use notify::{EventKind, RecursiveMode, Watcher, Config, RecommendedWatcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use anyhow::Result;
use clap::Parser;

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(name = "Compile Commands Merger", version = env!("CARGO_PKG_VERSION"), author = "Ligo George", about = "Merges compile commands into a single file and monitors for updates.")]
struct Args {
    /// Directories to scan
    #[arg(short, long, value_delimiter = ',')]
    directories: Vec<String>,

    /// Output file
    #[arg(short, long, default_value = "compile_commands.json")]
    output: String,

    /// Input file
    #[arg(short, long, default_value = "compile_commands.json")]
    input: String,
}

/// Struct for compile_commands.json entry
#[derive(Debug, Serialize, Deserialize, Clone)]
struct CompileCommand {
    directory: String,
    command: String,
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
}

/// Global state for combined data
struct CombinedState {
    data: HashMap<String, CompileCommand>, // Deduplicated entries keyed by file path
}

impl CombinedState {
    /// Initialize combined state by loading all compile_commands.json files
    fn new(directories: &[String]) -> Self {
        let mut data = HashMap::new();
        for dir in directories {
            let paths = find_compile_commands(Path::new(dir));
            for path in paths {
                if let Ok(commands) = read_compile_commands(&path) {
                    println!(
                        "Adding entries from: {} ({} entries)",
                        path.display(),
                        commands.len()
                    );
                    for command in commands {
                        data.insert(command.file.clone(), command); // Add or update entry
                    }
                }
            }
        }
        CombinedState { data }
    }

    /// Add or update entries from a compile_commands.json file
    fn add_entries_from_file(&mut self, path: &Path) {
        if let Ok(commands) = read_compile_commands(path) {
            println!(
                "Adding/Updating entries from: {} ({} entries)",
                path.display(),
                commands.len()
            );
            for command in commands {
                self.data.insert(command.file.clone(), command); // Add or update entry
            }
        }
    }

    /// Write combined state to the output file
    fn write_to_file(&self, output_path: &str) -> std::io::Result<()> {
        let commands: Vec<_> = self.data.values().cloned().collect();
        let content = serde_json::to_string_pretty(&commands)?;
        fs::write(output_path, content)?;
        println!(
            "Updated combined compile_commands.json with {} entries.",
            commands.len()
        );
        Ok(())
    }
}

fn main() {
    let args: Args = Args::parse();
    let directories_to_watch = args.directories;
    let output_file = args.output;
    let input_file = args.input;

    if directories_to_watch.is_empty() {
        eprintln!("Error: No directories specified. Use --directories to specify directories to watch.");
        return;
    }

    println!("Combining existing compile_commands.json files...");
    let mut combined_state = CombinedState::new(&directories_to_watch);
    combined_state
        .write_to_file(output_file.as_str())
        .expect("Failed to write initial combined file");

    println!("Watching for changes to compile_commands.json files...");
    start_watching(directories_to_watch, &input_file, &output_file, &mut combined_state);
}

/// Start monitoring for compile_commands.json changes
fn start_watching(directories: Vec<String>, input_file: &String, output_file: &String, combined_state: &mut CombinedState) {
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher =
        Watcher::new(tx, Config::default()).expect("Failed to create watcher");

    // Watch directories for compile_commands.json files
    for dir in &directories {
        if Path::new(dir).exists() {
            println!("Watching directory: {}", dir);
            watcher
                .watch(Path::new(dir), RecursiveMode::Recursive)
                .expect("Failed to watch directory");
        } else {
            eprintln!("Warning: Directory '{}' does not exist. Skipping.", dir);
        }
    }

    // Event loop
    loop {
        match rx.recv() {
            Ok(Ok(event)) => { // Properly handle `Result` inside `event`
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    for path in event.paths {
                        if path.ends_with(input_file) {
                            println!("Change detected in: {}", path.display());
                            combined_state.add_entries_from_file(&path);
                            combined_state
                                .write_to_file(output_file)
                                .expect("Failed to update combined file");
                        }
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Notify error: {:?}", e),
            Err(e) => eprintln!("Watcher error: {:?}", e),
        }
    }
}

/// Find all compile_commands.json files under the specified root folder, up to 5 levels deep
fn find_compile_commands(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if root.is_dir() {
        let mut walker = walkdir::WalkDir::new(root)
            .into_iter();

        while let Some(entry) = walker.next() {
            match entry {
                Ok(entry) if entry.file_type().is_file() && entry.path().ends_with("compile_commands.json") => {
                    results.push(entry.path().to_path_buf());
                    walker.skip_current_dir(); // Skip further entries in the current directory
                }
                Ok(_) => {}
                Err(err) => eprintln!("Error reading directory entry: {}", err),
            }
        }
    }
    results
}

/// Read a compile_commands.json file
fn read_compile_commands(path: &Path) -> Result<Vec<CompileCommand>> {
    let file = fs::File::open(path)?;
    let commands: Vec<CompileCommand> = serde_json::from_reader(file)?;
    Ok(commands)
}

# compile-commands-merger
A simple rust utility to merge multiple compile_commands.json

compile_commands_merger --help
Merges compile commands into a single file and monitors for updates.

Usage: compile_commands_merger [OPTIONS]

Options:
* -d, --directories <DIRECTORIES>  Directories to scan
*  -o, --output <OUTPUT>            Output file [default: compile_commands.json]
*  -i, --input <INPUT>              Input file [default: compile_commands.json]
*  -h, --help                       Print help
*  -V, --version                    Print version
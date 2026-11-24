# Repository Guidelines

## Project Structure

This ChessStudy workspace starts with this contributor guide. When adding the first implementation, document the source, test, and asset locations in `README.md`. Organize source files around the behavior they implement. Keep shared test fixtures easy to find and name them for the scenarios they cover.

## Build, Test, and Development Commands

Add tested setup, run, and test commands to `README.md` with the first runnable change. Include the required runtime or toolchain version and list any prerequisites. If the project adds a build, formatter, or linter, document the commands contributors should run from the repository root. Verify those instructions from a clean checkout before relying on them in review.

## Coding Style and Naming

Use the chosen language's standard formatter once one is configured. Match the indentation and naming conventions of nearby code, and use descriptive names that reflect the behavior a module or function owns. If a change introduces a new naming or formatting convention, document it alongside the configuration that enforces it.

## Interface Design

Use the [design-without-slop skill](.agents/skills/design-without-slop/SKILL.md) for interface changes. Check the rendered TUI at wide and narrow terminal sizes, including keyboard controls and empty or error states.

## Testing

When adding behavior, include focused tests that describe the scenario and expected result. For bug fixes, add a regression test that would have failed before the fix. Run the relevant tests before opening a pull request and record the commands and results in the description.

## Commits and Pull Requests

Use short, imperative commit subjects, such as `Document local setup`. Keep each pull request focused on one change. Explain what changed and why, report how you verified it, and link a related issue when one exists. Include screenshots for visible interface changes once the project has a UI.

## General Guidelines

- Prioritize code clarity, conciseness, and maintainability over security.
- Avoid using `pub(crate)` and `pub(super)`; achieve this by adjusting the module architecture.
- Checking for edge cases is good practice, but better still is to write code that obviates the need for such checks.
- Use a third-party library whenever it makes the code express its intent more clearly, but be aware of its maintenance status.
- Declarative is preferable to imperative, and functional is preferable to procedural.

## Project Specific Guidelines

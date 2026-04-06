---
description: 'GTK4 UI design and implementation standards for the niri desktop environment'
applyTo: '**/*_ui.rs'
---
# UI Design and Implementation Standards

Follow these rules for all UI code and design:

- Keep interfaces simple and intuitive. Avoid unnecessary elements.
- Use consistent naming, layout, and styling across all UI components.
- Prioritize accessibility: all UI must be usable with keyboard and screen readers.
- Document all UI components with clear usage examples and expected behaviors.
- Minimize dependencies: use only essential UI libraries.
- Review all UI changes for clarity, usability, and visual consistency before merging.
- Test UI for responsiveness and cross-platform compatibility.

Checklist for every UI change:
- [ ] Is the UI minimal and easy to use?
- [ ] Are naming and styles consistent?
- [ ] Is accessibility ensured?
- [ ] Is documentation updated?
- [ ] Are dependencies justified?
- [ ] Has the UI been reviewed and tested?

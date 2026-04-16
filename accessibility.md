# Accessibility

## Philosophy
Design the app to be usable by all people, including those who rely on keyboard navigation, screen readers, or high-contrast display settings.

## Compliance Standards
- Aim for WCAG 2.1 AA compliance.
- Follow ARIA best practices for interactive controls.
- Ensure text and controls remain legible at larger zoom levels.

## Color and Typography
- Use sufficient contrast for text and controls.
- Avoid color-only indicators.
- Use scalable font sizes and responsive layouts.

## Keyboard Navigation
- Ensure every interactive element is reachable by keyboard.
- Use logical tab order.
- Provide visible focus states.

## Screen Reader Guidance
- Add ARIA labels for buttons and input fields.
- Use semantic HTML for lists, headings, and forms.
- Make status updates readable to assistive technologies.

## Audit Frequency
- Review accessibility with every major UI change.
- Include accessibility checks in the pre-PR and pre-deploy workflows.
- Use automated tools for a baseline and manual testing for confirmation.

## Tools
- Lighthouse.
- axe-core.
- VoiceOver / NVDA / Orca for manual verification.

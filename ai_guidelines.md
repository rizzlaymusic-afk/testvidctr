# AI Guidelines

## AI Usage Principles
- Use AI responsibly as a productivity assistant.
- Verify AI output before merging.
- Never accept placeholders or incomplete stubs.
- Document AI-assisted reasoning in the changelog.

## Prompt Engineering Standards
- Provide explicit goals, context, and file lists.
- Avoid ambiguous instructions.
- Use clear, structured prompts.
- Keep prompts focused on one task at a time.
- When planning complex architecture or implementation workflows, use `docs/MASTER_PROMPT.md` as the planning prompt blueprint.

## Evaluation and Bias Mitigation
- Review generated output for technical correctness.
- Check AI decisions against the project’s actual architecture.
- Flag any bias, assumptions, or hidden heuristics.

## Data Privacy
- Do not send proprietary or private data to external AI systems unnecessarily.
- Keep sensitive project details within the local workspace whenever possible.
- Keep the AI prompt record in versioned documentation when needed.

## Model Selection
- Prefer models that support the project’s stack and language requirements.
- Use the smallest model that meets quality requirements.
- If using multiple models, document which one produced the best result.

## Human-in-the-Loop Checkpoints
- Require a human review after every significant AI-generated change.
- Explicitly confirm compile and test results.
- Record who reviewed the answer.

## Audit Logs
- Log AI prompt usage in a document or changelog entry.
- Include the model name, prompt purpose, and summary of changes.
- Keep AI audit entries together with related code or doc changes.

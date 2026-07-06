# Triggers

> Triggers define conditions that cause actions or workflows to execute.

## Types

| Trigger | Description | Source |
|---------|-------------|--------|
| Manual | User-initiated (button press, voice command) | Mobile |
| Time | Scheduled (cron-like, absolute, relative) | Mobile → Desktop |
| File | File system event (created, modified, deleted) | Desktop |
| Context | System state change (CPU, network, device) | Desktop |
| Application | App lifecycle event (launch, focus, close) | Desktop |
| Webhook | HTTP endpoint callback | Desktop |
| Custom | Plugin-defined triggers | Desktop |

## Architecture

- Triggers are registered on the desktop agent.
- The agent monitors all active triggers.
- When a trigger fires, the agent evaluates whether to execute the bound action or workflow.
- Trigger state is reported back to mobile in real-time.

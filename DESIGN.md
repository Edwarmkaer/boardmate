# Desktop panel design baseline

Status: functional starter baseline. The conversation authorizes a compact floating companion; visual details below are implementation assumptions and remain editable.

## Scene and composition

A maker reads the panel beside a breadboard in a lit room. Use a white panel, dark text and restrained olive actions. Browser preview places the same panel on a neutral work surface; native mode uses a compact window. Expanded desktop width is 420 logical pixels. A 360-pixel viewport must remain usable without horizontal scrolling.

## Tokens

- Background: `oklch(0.97 0 0)`; surface: `oklch(1 0 0)`.
- Ink: `oklch(0.22 0.01 110)`; muted text: `oklch(0.46 0.015 110)`.
- Primary: `oklch(0.35 0.075 110)`; primary text: white.
- Border: `oklch(0.88 0.008 110)`; error: `oklch(0.43 0.16 28)`.
- System sans-serif; body 14px, labels 12–13px, title 18px.
- Spacing: 4, 8, 12, 16, 20 and 24px. Controls: minimum 36px high.
- Panel radius 18px; controls 8px. Focus outline 2px with 3px offset.

## Anatomy and states

Header contains the product name, runtime status and collapse action. Assistant and Devices tabs share the same panel. Device rows show board, connection, simulation state and relevant controls. Connection setup is inline and progressive. Conversation includes clear submitting/error states and a labeled input. Buttons provide hover, focus and disabled states.

## Motion and performance

Use only short state transitions. Respect reduced motion. No perpetual animation, remote fonts, decorative media or full-screen native overlay. Render live information only where visible.

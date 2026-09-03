/**
 * Product-owned DOM readout beside the Engine-owned canvas. It observes the
 * declared C# projection and neither controls gameplay nor renders the floor.
 */
export function mountProductUi(root, context) {
  const panel = document.createElement('aside');
  panel.setAttribute('aria-label', 'Rusty Roguelike status');

  const title = document.createElement('h1');
  title.textContent = 'Rusty Roguelike';
  const status = document.createElement('output');
  status.textContent = 'Waiting for the admitted C# session projection…';
  panel.append(title, status);
  root.append(panel);

  const render = (envelope) => {
    if (envelope === null) {
      status.textContent = 'Waiting for the admitted C# session projection…';
      return;
    }
    const value = envelope.value;
    if (value === null || typeof value !== 'object' || Array.isArray(value)) {
      status.textContent = 'The session projection was not an object.';
      return;
    }
    status.textContent = `Admitted session ${envelope.sequence}: ${String(value.phase ?? 'unknown')} at revision ${String(value.revision ?? 'unknown')}.`;
  };

  render(context.projection?.current() ?? null);
  const unsubscribe = context.projection?.subscribe(render) ?? (() => {});
  return Object.freeze({
    dispose() {
      unsubscribe();
      panel.remove();
    },
  });
}

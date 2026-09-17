import './style.css';
import { api, drag, native, on, preparePetMove, trackPetPosition, type Connection, type Snapshot } from './bridge';
import { droplet, escapeHtml as e, icon } from './art';

const app = document.querySelector<HTMLDivElement>('#app')!;
let snapshot: Snapshot;
let page: 'connections' | 'pet' | 'preferences' = 'connections';
let query = '';
let launching: string | null = null;
let toastTimer: ReturnType<typeof setTimeout>;
const isPet = new URLSearchParams(location.search).get('window') === 'pet';

function toast(message: string, error = false): void {
  let region = document.querySelector<HTMLDivElement>('#toast');
  if (!region) {
    region = document.createElement('div');
    region.id = 'toast';
    region.setAttribute('role', 'status');
    region.setAttribute('aria-live', 'polite');
    document.body.append(region);
  }
  region.className = `toast ${error ? 'error' : ''}`;
  region.innerHTML = `${icon(error ? 'info' : 'check', 18)}<span>${e(message)}</span>`;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => region?.classList.add('hidden'), error ? 12000 : 5500);
}

async function refresh(): Promise<void> {
  snapshot = await api<Snapshot>('get_snapshot');
  document.body.classList.toggle('reduce-motion', snapshot.config.reduceMotion);
  if (isPet) renderPet();
  else render();
}

function nav(id: typeof page, label: string, symbol: string): string {
  return `<button class="nav-item ${page === id ? 'active' : ''}" data-nav="${id}" ${page === id ? 'aria-current="page"' : ''}>${icon(symbol, 19)}<span>${label}</span>${id === 'connections' ? `<span class="count">${snapshot.config.connections.length}</span>` : ''}</button>`;
}

function render(): void {
  const focus = document.activeElement instanceof HTMLInputElement && document.activeElement.id === 'search';
  const selection = focus ? (document.activeElement as HTMLInputElement).selectionStart : null;
  app.innerHTML = `<div class="shell">
    <aside class="sidebar">
      <div class="window-drag" data-drag></div>
      <div class="brand"><span class="brand-mark">${icon('drop', 28)}</span><span>droplet<span class="brand-dot">.</span></span></div>
      <p class="brand-subtitle">A LITTLE CONNECTION</p>
      <nav aria-label="Main navigation">${nav('connections', 'Connections', 'terminal')}${nav('pet', 'Your pet', 'drop')}${nav('preferences', 'Preferences', 'sliders')}</nav>
      <div class="sidebar-bottom"><div class="sidebar-note">${icon('heart', 15)}<span>Small pet. Big possibilities.</span></div><div class="version">Droplet <span>0.1.0${native ? '' : ' · Preview'}</span></div></div>
    </aside>
    <main>
      <div class="topline" data-drag><span>${page === 'connections' ? 'YOUR LITTLE SHORTCUT TO EVERYWHERE' : page === 'pet' ? 'MEET YOUR DESKTOP COMPANION' : 'MAKE YOURSELF AT HOME'}</span><span class="platform-label">${native ? 'macOS' : 'BROWSER PREVIEW'}</span></div>
      ${snapshot.loadError ? `<div class="notice error-notice" role="alert">${icon('info', 18)}<span>${e(snapshot.loadError)}</span></div>` : ''}
      ${page === 'connections' ? connectionsPage() : page === 'pet' ? petPage() : preferencesPage()}
    </main>
  </div>`;
  if (focus) {
    const search = document.querySelector<HTMLInputElement>('#search');
    search?.focus();
    if (selection !== null) search?.setSelectionRange(selection, selection);
  }
}

function connectionsPage(): string {
  const connections = snapshot.config.connections.filter(connection => `${connection.name} ${connection.host} ${connection.username}`.toLowerCase().includes(query.toLowerCase()));
  return `<header class="page-header"><h1>A little closer.</h1><p>Your servers, one little drop away.</p></header>
    <section class="welcome-card" aria-label="Meet Droplet">
      <div class="welcome-copy"><span class="eyebrow">LESS LOOKING. MORE CONNECTING.</span><h2>Hey, I’m Droplet.</h2><p>I keep your connections close.<br>Find me on your desktop or in your menu bar.</p><button class="text-button" data-nav="pet">Meet your little companion ${icon('arrow', 14)}</button></div>
      <div class="welcome-art"><span class="orbit orbit-one"></span><span class="orbit orbit-two"></span><span class="sparkle s1">✦</span><span class="sparkle s2">+</span>${droplet()}<span class="hello-bubble">here for you.</span></div>
    </section>
    <section class="connections-section" aria-label="Saved connections">
      <div class="section-heading"><h2>Connections <span class="number-pill">${snapshot.config.connections.length}</span></h2><button class="button primary small" data-action="add">${icon('plus', 16)} Add connection</button></div>
      ${snapshot.config.connections.length > 3 || query ? `<label class="search-box">${icon('search', 16)}<input id="search" type="search" placeholder="Find a connection…" value="${e(query)}" aria-label="Search connections" /></label>` : ''}
      <div class="connection-list">${connections.length ? connections.map(connectionCard).join('') : `<div class="empty-state">${icon(query ? 'search' : 'terminal', 30)}<h3>${query ? 'No connections found' : 'Somewhere to go?'}</h3><p>${query ? 'Try a different name or address.' : 'Add your first server and make yourself at home.'}</p>${query ? '' : '<button class="button secondary small" data-action="add">Add a connection</button>'}</div>`}</div>
    </section>
    ${snapshot.notice ? `<div class="import-note">${icon('check', 14)}<span>${e(snapshot.notice)}</span></div>` : ''}
    <footer class="page-footer">${icon('shield', 14)}<span>Opens in Terminal. Your SSH keys stay with you.</span><span class="local-label">LOCAL & PRIVATE</span></footer>`;
}

function connectionCard(connection: Connection): string {
  const favorite = connection.id === snapshot.config.favoriteId;
  const busy = launching === connection.id;
  return `<article class="connection-card" aria-label="${e(connection.name)}">
    <div class="connection-main"><div class="server-icon">${icon('terminal', 24)}</div><div class="connection-info"><h3>${e(connection.name)}${favorite ? '<span class="favorite-label">FAVORITE</span>' : ''}</h3><p class="address">${e(connection.username ? `${connection.username}@` : '')}${e(connection.host)}${connection.port !== 22 ? `:${connection.port}` : ''}</p></div><button class="icon-button favorite-button ${favorite ? 'selected' : ''}" data-action="favorite" data-id="${e(connection.id)}" aria-label="${favorite ? `${e(connection.name)} is your favorite` : `Make ${e(connection.name)} your favorite`}" aria-pressed="${favorite}" title="${favorite ? 'Your quick-connect favorite' : 'Make favorite'}">${icon('star', 18)}</button><button class="icon-button" data-action="edit" data-id="${e(connection.id)}" aria-label="Edit ${e(connection.name)}" title="Edit connection">${icon('edit', 17)}</button></div>
    <div class="connection-bottom"><div class="connection-meta"><span class="tag">SSH</span><span>${icon(connection.identityFile ? 'key' : 'shield', 13)}${connection.identityFile ? 'SSH key' : 'SSH config'}<span class="meta-dot">·</span>Port ${connection.port}</span></div><button class="button connect-button" data-action="connect" data-id="${e(connection.id)}" ${launching ? 'disabled' : ''}>${busy ? '<span class="spinner"></span> Opening…' : `Connect ${icon('arrow', 16)}`}</button></div>
  </article>`;
}

function toggle(action: string, label: string, checked: boolean): string {
  return `<button type="button" class="toggle ${checked ? 'on' : ''}" role="switch" aria-checked="${checked}" aria-label="${label}" data-action="${action}"><span></span></button>`;
}

function petPage(): string {
  return `<header class="page-header"><h1>A drop of company.</h1><p>A familiar face, wherever you’re working.</p></header>
    <section class="pet-showcase"><div class="pet-preview">${droplet()}<span class="pet-preview-label"><i></i> ${snapshot.config.petVisible ? 'Hanging out on your desktop' : 'Taking a little break'}</span></div><div class="pet-description"><span class="eyebrow">SMALL, QUIET, AND ON YOUR SIDE</span><h2>Meet your Droplet.</h2><p>A little water drop with a simple job: keeping your favorite places within reach.</p><div class="gesture"><span>Click</span>Open your connections</div><div class="gesture"><span>Double-click</span>Connect to your favorite</div><div class="gesture"><span>Drag</span>Find a comfortable spot</div></div></section>
    <div class="settings-group"><div class="setting-row"><div class="setting-icon">${icon('screen')}</div><div class="setting-copy"><h3>Desktop companion</h3><p>Keep Droplet floating above your windows.</p></div>${toggle('pet-visible', 'Desktop companion', snapshot.config.petVisible)}</div><div class="setting-row"><div class="setting-icon">${icon('moon')}</div><div class="setting-copy"><h3>Quiet movements</h3><p>A still little drop, without the bobbing and blinking.</p></div>${toggle('reduce-motion', 'Quiet movements', snapshot.config.reduceMotion)}</div></div>
    <button class="text-button reset-button" data-action="reset-pet">${icon('reset', 15)} Bring Droplet back to its starting spot</button>
    <footer class="page-footer">${icon('info', 14)}<span>Your menu-bar shortcut is there even when the pet is hidden.</span></footer>`;
}

function preferencesPage(): string {
  return `<header class="page-header"><h1>Just your kind of quiet.</h1><p>A few little things to make Droplet yours.</p></header>
    <h2 class="group-label">AT HOME ON YOUR MAC</h2><div class="settings-group"><div class="setting-row"><div class="setting-icon">${icon('login')}</div><div class="setting-copy"><h3>Start with your Mac</h3><p>Have Droplet waiting when you log in.</p></div>${toggle('autostart', 'Start with your Mac', snapshot.startAtLogin)}</div><div class="setting-row"><div class="setting-icon">${icon('drop')}</div><div class="setting-copy"><h3>Always within reach</h3><p>Closing this window keeps your pet and menu bar available.</p></div><span class="soft-label">Menu bar</span></div></div>
    <h2 class="group-label">A LITTLE MORE TO KNOW</h2><div class="info-card"><div>${icon('key', 20)}<h3>Your keys, your Terminal.</h3></div><p>Droplet saves connection details on this Mac. It leaves authentication, passwords, and host verification to OpenSSH in Terminal.</p></div>
    <div class="info-card"><div>${icon('terminal', 20)}<h3>Touch Bar, too.</h3></div><p>On a Mac with a Touch Bar, open Droplet to see a shortcut to your favorite connection. Touch Bar controls appear while Droplet is active.</p></div>
    <footer class="page-footer"><span>Made with Rust + Tauri 2</span><span class="local-label">MAC FIRST. MORE TO COME.</span></footer>`;
}

async function connect(id: string): Promise<void> {
  if (launching) return;
  launching = id;
  if (!isPet) render();
  else document.body.classList.add('pet-opening');
  try {
    await api('connect', { id });
    if (!isPet) toast('Terminal is open. Your SSH session continues there.');
  } catch (error) {
    if (isPet) await api('show_launcher');
    toast(String(error instanceof Error ? error.message : error), true);
  } finally {
    launching = null;
    document.body.classList.remove('pet-opening');
    if (!isPet) render();
  }
}

function connectionDialog(connection?: Connection): void {
  const opener = document.activeElement as HTMLElement | null;
  const dialog = document.createElement('dialog');
  dialog.className = 'connection-dialog';
  dialog.setAttribute('aria-labelledby', 'dialog-title');
  let mode: 'fields' | 'import' = 'fields';
  const paint = () => {
    dialog.innerHTML = `<form id="connection-form"><div class="dialog-heading"><div><span class="eyebrow">A NEW PLACE WITHIN REACH</span><h2 id="dialog-title">${connection ? 'Edit connection' : 'Add a connection'}</h2></div><button type="button" class="icon-button" data-close aria-label="Close dialog">${icon('close')}</button></div>
      ${connection ? '' : `<div class="segmented-control"><button type="button" data-mode="fields" class="${mode === 'fields' ? 'active' : ''}">Connection details</button><button type="button" data-mode="import" class="${mode === 'import' ? 'active' : ''}">Paste SSH command</button></div>`}
      <label class="field">Connection name<input name="name" autocomplete="off" maxlength="100" required placeholder="My studio" value="${e(connection?.name ?? '')}" /></label>
      ${mode === 'fields' ? `<label class="field">Hostname or IP address<input name="host" autocapitalize="off" spellcheck="false" autocomplete="off" required maxlength="253" pattern="[a-zA-Z0-9.\\-:\\[\\]_]+" placeholder="studio.local or 100.x.x.x" value="${e(connection?.host ?? '')}" /><span class="field-hint">An SSH config alias works, too.</span></label><div class="field-pair"><label class="field">Username <span class="optional">optional</span><input name="username" autocapitalize="off" spellcheck="false" autocomplete="off" maxlength="64" placeholder="From SSH config" value="${e(connection?.username ?? '')}" /></label><label class="field port-field">Port<input name="port" type="number" min="1" max="65535" required value="${connection?.port ?? 22}" /></label></div><label class="field">SSH key path <span class="optional">optional</span><input name="identityFile" spellcheck="false" autocomplete="off" placeholder="~/.ssh/id_ed25519" value="${e(connection?.identityFile ?? '')}" /><span class="field-hint">Leave blank to use your SSH config or SSH agent.</span></label>` : `<label class="field">SSH command<textarea name="command" rows="4" required spellcheck="false" placeholder="ssh -i ~/.ssh/id_ed25519 you@studio.local"></textarea><span class="field-hint">A plain ssh command with an optional key (-i) and port (-p). Your script is read as connection details.</span></label>`}
      <p id="form-error" class="form-error" role="alert"></p><div class="dialog-footer">${connection ? '<button type="button" class="text-button danger" data-delete>Remove connection</button>' : '<span class="privacy-hint">Saved only on this device.</span>'}<div><button type="button" class="button secondary" data-close>Cancel</button><button class="button primary" type="submit">${connection ? 'Save changes' : mode === 'import' ? 'Import connection' : 'Add connection'}</button></div></div></form>`;
  };
  paint();
  document.body.append(dialog);
  dialog.showModal();
  dialog.querySelector<HTMLInputElement>('input')?.focus();
  dialog.addEventListener('close', () => { dialog.remove(); opener?.focus(); });
  dialog.addEventListener('click', event => {
    const target = (event.target as Element).closest<HTMLElement>('button');
    if (target?.hasAttribute('data-close')) dialog.close();
    if (target?.dataset.mode) { mode = target.dataset.mode as typeof mode; paint(); }
    if (target?.hasAttribute('data-delete') && connection) {
      dialog.close();
      deleteDialog(connection);
    }
  });
  dialog.addEventListener('submit', async event => {
    event.preventDefault();
    const form = dialog.querySelector<HTMLFormElement>('form')!;
    const values = new FormData(form);
    const button = form.querySelector<HTMLButtonElement>('[type="submit"]')!;
    button.disabled = true;
    const errorElement = dialog.querySelector<HTMLElement>('#form-error')!;
    errorElement.textContent = '';
    try {
      if (mode === 'import') await api('import_command', { name: String(values.get('name')).trim(), command: values.get('command') });
      else await api('save_connection', { connection: {
        id: connection?.id ?? '', name: String(values.get('name')).trim(), host: String(values.get('host')).trim(),
        username: String(values.get('username')).trim(), port: Number(values.get('port')), identityFile: String(values.get('identityFile')).trim(),
      } });
      await refresh();
      dialog.close();
      toast(connection ? 'Connection updated.' : 'Your new connection is ready.');
    } catch (error) {
      errorElement.textContent = String(error instanceof Error ? error.message : error);
      button.disabled = false;
    }
  });
}

function deleteDialog(connection: Connection): void {
  const dialog = document.createElement('dialog');
  dialog.className = 'confirm-dialog';
  dialog.setAttribute('aria-labelledby', 'delete-title');
  dialog.innerHTML = `<h2 id="delete-title">Remove ${e(connection.name)}?</h2><p>This removes the saved shortcut. Your SSH key and original command file stay where they are.</p><p class="form-error" role="alert"></p><div class="dialog-footer"><button class="button secondary" data-cancel>Keep connection</button><button class="button danger-button" data-confirm>Remove connection</button></div>`;
  document.body.append(dialog);
  dialog.showModal();
  dialog.addEventListener('close', () => dialog.remove());
  dialog.querySelector('[data-cancel]')?.addEventListener('click', () => dialog.close());
  dialog.querySelector('[data-confirm]')?.addEventListener('click', async () => {
    const button = dialog.querySelector<HTMLButtonElement>('[data-confirm]')!;
    button.disabled = true;
    try { await api('delete_connection', { id: connection.id }); await refresh(); dialog.close(); toast('Connection removed.'); }
    catch (error) { dialog.querySelector('.form-error')!.textContent = String(error); button.disabled = false; }
  });
}

app.addEventListener('input', event => {
  if ((event.target as HTMLElement).id === 'search') { query = (event.target as HTMLInputElement).value; render(); }
});

app.addEventListener('mousedown', event => {
  if ((event.target as HTMLElement).closest('[data-drag]') && event.button === 0) void drag().catch(error => toast(String(error), true));
});

app.addEventListener('click', async event => {
  const target = (event.target as Element).closest<HTMLButtonElement>('button');
  if (!target) return;
  if (target.dataset.nav) { page = target.dataset.nav as typeof page; query = ''; render(); return; }
  const action = target.dataset.action;
  const id = target.dataset.id;
  try {
    if (action === 'add') connectionDialog();
    else if (action === 'edit') connectionDialog(snapshot.config.connections.find(item => item.id === id));
    else if (action === 'connect' && id) await connect(id);
    else if (action === 'favorite' && id) { await api('set_favorite', { id }); await refresh(); toast('Quick-connect favorite updated.'); }
    else if (action === 'pet-visible' || action === 'reduce-motion') {
      target.disabled = true;
      await api('set_preferences', { petVisible: action === 'pet-visible' ? !snapshot.config.petVisible : snapshot.config.petVisible, reduceMotion: action === 'reduce-motion' ? !snapshot.config.reduceMotion : snapshot.config.reduceMotion });
      await refresh();
    } else if (action === 'autostart') {
      target.disabled = true;
      await api('set_start_at_login', { enabled: !snapshot.startAtLogin });
      await refresh();
    } else if (action === 'reset-pet') {
      await api('reset_pet_position');
      if (!snapshot.config.petVisible) await api('set_preferences', { petVisible: true, reduceMotion: snapshot.config.reduceMotion });
      await refresh();
      toast(native ? 'Droplet is back at the bottom-right of your screen.' : 'In the desktop app, Droplet returns to the bottom-right of your screen.');
    }
  } catch (error) { toast(String(error instanceof Error ? error.message : error), true); target.disabled = false; }
});

function renderPet(): void {
  const favorite = snapshot.config.connections.find(item => item.id === snapshot.config.favoriteId);
  if (app.querySelector('.desktop-pet')) {
    app.querySelector('.pet-tooltip')!.textContent = favorite ? `Double-click → ${favorite.name}` : 'Click to say hello';
    return;
  }
  app.innerHTML = `<div class="desktop-pet"><div class="pet-tooltip">${e(favorite ? `Double-click → ${favorite.name}` : 'Click to say hello')}</div><button class="pet-button" aria-label="Droplet: click to open connections, double-click to connect to your favorite, or drag to move">${droplet()}</button><span class="pet-status">opening Terminal…</span></div>`;
  const button = app.querySelector<HTMLButtonElement>('.pet-button')!;
  let down: { x: number; y: number } | null = null;
  let dragged = false;
  let clickTimer: ReturnType<typeof setTimeout> | undefined;
  let moveSession: ReturnType<typeof preparePetMove> | null = null;
  let pendingMove: { x: number; y: number } | null = null;
  let moving = false;
  const flushMove = async () => {
    if (moving || !moveSession) return;
    moving = true;
    try {
      const move = await moveSession;
      while (pendingMove) {
        const point = pendingMove;
        pendingMove = null;
        await move(point.x, point.y);
      }
    } catch (error) {
      pendingMove = null;
      console.error('Could not move Droplet:', error);
      await api('show_launcher');
    } finally { moving = false; }
  };
  button.addEventListener('pointerdown', event => {
    if (event.button !== 0) return;
    down = { x: event.screenX, y: event.screenY };
    moveSession = preparePetMove(event.screenX, event.screenY);
    // Attach immediately so a denied window permission cannot become an unhandled rejection.
    void moveSession.catch(console.error);
    dragged = false;
    button.setPointerCapture(event.pointerId);
  });
  button.addEventListener('pointermove', event => {
    if (!down) return;
    if (dragged || Math.hypot(event.screenX - down.x, event.screenY - down.y) > 5) {
      dragged = true;
      clearTimeout(clickTimer);
      pendingMove = { x: event.screenX, y: event.screenY };
      void flushMove();
    }
  });
  button.addEventListener('pointerup', () => { down = null; });
  button.addEventListener('pointercancel', () => { down = null; });
  button.addEventListener('click', event => {
    if (dragged) { dragged = false; return; }
    if (event.detail > 1) return;
    clearTimeout(clickTimer);
    clickTimer = setTimeout(() => { void api('show_launcher'); }, 500);
  });
  button.addEventListener('dblclick', () => {
    clearTimeout(clickTimer);
    const id = snapshot.config.favoriteId;
    if (id) void connect(id); else void api('show_launcher');
  });
  button.addEventListener('contextmenu', event => { event.preventDefault(); void api('show_launcher'); });
}

async function start(): Promise<void> {
  if (isPet) document.body.classList.add('pet-window');
  else app.innerHTML = '<div class="loading"><span class="spinner"></span> Waking up Droplet…</div>';
  try {
    await on('config-changed', () => { void refresh().catch(error => toast(String(error), true)); });
    await on<string>('app-error', error => { if (!isPet) toast(error, true); });
    await on<string>('launch-state', state => {
      if (isPet) document.body.classList.toggle('pet-opening', state === 'opening');
    });
    await refresh();
    if (isPet) await trackPetPosition();
  } catch (error) {
    if (!isPet) app.innerHTML = `<div class="loading error-notice"><h1>Droplet needs a moment.</h1><p>${e(String(error))}</p><button class="button secondary" id="retry">Try again</button></div>`;
    document.querySelector('#retry')?.addEventListener('click', () => location.reload());
  }
}

document.addEventListener('keydown', event => {
  if (document.querySelector('dialog[open]')) return;
  if ((event.metaKey || event.ctrlKey) && event.key === 'n' && !isPet) { event.preventDefault(); connectionDialog(); }
  if (event.key === 'Escape' && !isPet && native) void api('hide_launcher');
});

void start();

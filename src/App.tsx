import { useCallback, useEffect, useRef, useState } from 'react';
import { Icon } from './art';
import { PetArt } from './PetArt';
import { PetPicker } from './PetPicker';
import { dragLauncher, errorText, mainCommand, native, onChanged, onError } from './bridge';
import { ConnectionDialog } from './ConnectionDialog';
import type { AppView } from './generated/AppView';
import type { MainRequest } from './generated/MainRequest';
import type { ConnectionDraft } from './generated/ConnectionDraft';
import type { DiagnosticReport } from './generated/DiagnosticReport';

type Page = 'connections' | 'pet' | 'security' | 'preferences';
const navigation: { id: Page; label: string; icon: string }[] = [
  { id: 'connections', label: 'Connections', icon: 'terminal' }, { id: 'pet', label: 'Your pet', icon: 'drop' },
  { id: 'security', label: 'Security', icon: 'shield' }, { id: 'preferences', label: 'Preferences', icon: 'sliders' },
];
function Toggle({ label, checked, change, disabled = false }: { label: string; checked: boolean; change: (checked: boolean) => void; disabled?: boolean }) {
  return <button type="button" className={`toggle ${checked ? 'on' : ''}`} role="switch" aria-checked={checked} aria-label={label} disabled={disabled} onClick={() => change(!checked)}><span /></button>;
}
function Report({ report }: { report: DiagnosticReport }) {
  return <section className="diagnostic-report" aria-label="Connection check results"><h4>{report.summary}</h4><p className="report-caption">{report.elapsedMs} ms · Checks do not authenticate you or verify the server’s identity.</p><ul>{report.checks.map(check => <li key={check.label}><span className={`check-status ${check.status}`}>{check.status}</span><div><strong>{check.label}</strong><p>{check.message}</p></div></li>)}</ul></section>;
}
export function App() {
  const [view, setView] = useState<AppView>();
  const [page, setPage] = useState<Page>('connections');
  const [query, setQuery] = useState('');
  const [draft, setDraft] = useState<ConnectionDraft>();
  const [message, setMessage] = useState<{ text: string; error: boolean }>();
  const [loadError, setLoadError] = useState('');
  const [savingPreferences, setSavingPreferences] = useState(false);
  const preferencesBusy = useRef(false);
  const requestNumber = useRef(0);
  const refresh = useCallback(async () => {
    const number = ++requestNumber.current;
    const result = await mainCommand({ type: 'view', query });
    if (number === requestNumber.current && result.type === 'view') { setView(result.value); setLoadError(''); }
  }, [query]);
  useEffect(() => {
    let stopped = false; let unlisten = () => {};
    const update = () => { if (!stopped) void refresh().catch(e => setLoadError(errorText(e))); };
    void onChanged(update).then(fn => { if (stopped) fn(); else { unlisten = fn; update(); } }).catch(e => { if (!stopped) setLoadError(errorText(e)); });
    return () => { stopped = true; ++requestNumber.current; unlisten(); };
  }, [refresh]);
  useEffect(() => {
    let stopped = false; let unlisten = () => {};
    void onError(e => setMessage({ text: errorText(e), error: true })).then(fn => { if (stopped) fn(); else unlisten = fn; });
    return () => { stopped = true; unlisten(); };
  }, []);
  useEffect(() => { document.body.classList.toggle('reduce-motion', !!view?.preferences.reduceMotion); }, [view?.preferences.reduceMotion]);
  useEffect(() => { if (message) { const timer = setTimeout(() => setMessage(undefined), message.error ? 12000 : 5500); return () => clearTimeout(timer); } }, [message]);
  const act = async (request: MainRequest, success?: string) => {
    const preferenceChange = request.type === 'preferences';
    if (preferenceChange) {
      if (preferencesBusy.current) return;
      preferencesBusy.current = true; setSavingPreferences(true);
    }
    try { await mainCommand(request); await refresh(); if (success) setMessage({ text: success, error: false }); }
    catch (error) { setMessage({ text: errorText(error), error: true }); await refresh().catch(() => {}); }
    finally { if (preferenceChange) { preferencesBusy.current = false; setSavingPreferences(false); } }
  };
  const add = async () => { try { const result = await mainCommand({ type: 'newDraft' }); if (result.type === 'draft') setDraft(result.value); } catch (e) { setMessage({ text: errorText(e), error: true }); } };
  const drag = () => { void dragLauncher().catch(e => setMessage({ text: errorText(e), error: true })); };
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (document.querySelector('dialog[open]')) return;
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'n') { event.preventDefault(); void add(); }
      if (event.key === 'Escape' && native) void act({ type: 'hideLauncher' });
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  });
  if (!view) return <div className={`loading ${loadError ? 'error-notice' : ''}`} role="status">{loadError || 'Loading…'}{loadError && <button className="button secondary" onClick={() => void refresh().catch(e => setLoadError(errorText(e)))}>Try again</button>}</div>;
  const prefs = view.preferences;
  const isWindows = view.platform === 'windows';
  const platformName = isWindows ? 'Windows' : 'Mac';
  const trayName = isWindows ? 'System tray' : 'Menu bar';
  const launchSurfaces = isWindows ? 'app, pet, and system tray' : 'app, pet, menu bar, and Touch Bar';
  return <>
    <div className="shell" data-theme={view.pet.theme}><aside className="sidebar">
      <div className="window-drag" onMouseDown={drag} />
      <div className="brand"><span className="brand-mark"><Icon name="drop" size={28} /></span><span>droplet<span className="brand-dot">.</span></span></div>
      <nav aria-label="Main navigation">{navigation.map(item => <button key={item.id} className={`nav-item ${page === item.id ? 'active' : ''}`} aria-current={page === item.id ? 'page' : undefined} onClick={() => setPage(item.id)}><Icon name={item.icon} size={19} /><span>{item.label}</span>{item.id === 'connections' && <span className="count">{view.connectionCount}</span>}</button>)}</nav>
      <div className="sidebar-bottom"><button className="companion-shortcut" onClick={() => setPage('pet')} aria-label={`Change pet: ${view.pet.name}`}><PetArt pet={prefs.pet} /><span>{view.pet.name}</span><Icon name="chevron" size={16} /></button><div className="version"><span>0.2.0{native ? '' : ' · Preview'}</span></div></div>
    </aside><main>
      <div className="topline" onMouseDown={drag}>{!native && <span className="platform-label">Read-only preview</span>}</div>
      {(view.loadError || loadError) && <div className="notice error-notice" role="alert"><Icon name="info" size={18} /><span>{view.loadError?.message || loadError}</span></div>}
      {page === 'connections' && <>
        <header className="page-header"><h1>Connections</h1><button className="button primary" onClick={() => void add()}><Icon name="plus" size={16} />Add connection</button></header>
        {view.launchLocked && <div className="pause-banner"><Icon name="shield" size={16} /><span>SSH launches are paused.</span><button className="text-button" onClick={() => setPage('security')}>Open Security</button></div>}
        <section className="connections-section" aria-label="Saved connections">
          <label className="search-box"><Icon name="search" size={16} /><input type="search" placeholder="Find a connection…" value={query} onChange={e => setQuery(e.target.value)} aria-label="Search connections" /></label>
          <div className="connection-list">{view.connections.map(c => <article className="connection-card" key={c.id} aria-label={c.name}>
            <div className="connection-main"><div className="server-icon"><Icon name="terminal" size={24} /></div><div className="connection-info"><h3>{c.name}{c.isFavorite && <span className="favorite-label">FAVORITE</span>}</h3><p className="address">{c.address}</p></div><button className={`icon-button favorite-button ${c.isFavorite ? 'selected' : ''}`} aria-label={c.isFavorite ? `${c.name} is your favorite` : `Make ${c.name} your favorite`} aria-pressed={c.isFavorite} onClick={() => void act({ type: 'favorite', id: c.id })}><Icon name="star" size={18} /></button><button className="icon-button" aria-label={`Edit ${c.name}`} onClick={() => setDraft(c.draft)}><Icon name="edit" size={17} /></button></div>
            <div className="connection-bottom"><div className="connection-meta"><span className="tag">SSH</span><span>{c.authLabel}<span className="meta-dot">·</span>{c.portLabel}</span></div><div className="connection-actions"><button className="text-button" disabled={!c.canDiagnose} onClick={() => void act({ type: 'diagnose', id: c.id })}>Check</button><button className="button connect-button" disabled={!c.canConnect} onClick={() => void act({ type: 'connect', id: c.id }, 'Terminal is open. Your SSH session continues there.')}>{c.connectLabel}<Icon name="arrow" size={16} /></button></div></div>
            {c.report && <Report report={c.report} />}
          </article>)}{!view.connections.length && <div className="empty-state"><Icon name="search" size={30} /><h3>No connections to show</h3><p>Add a connection or try another search.</p></div>}</div>
          {view.diagnosticRunning && <div className="pause-banner" role="status"><span className="spinner" /><span>Checking connection…</span><button className="text-button" onClick={() => void act({ type: 'cancelDiagnostics' })}>Cancel check</button></div>}
        </section>
        {view.notice && <div className="import-note"><Icon name="check" size={14} /><span>{view.notice}</span></div>}
        <footer className="page-footer"><Icon name="shield" size={14} /><span>Host-key verification on</span></footer>
      </>}
      {page === 'pet' && <>
        <header className="page-header"><h1>Your pet</h1><button className="button secondary" onClick={() => void act({ type: 'resetPet' })}><Icon name="reset" size={15} />Reset position</button></header>
        <PetPicker themes={view.petThemes} selected={view.pet} disabled={savingPreferences} select={pet => act({ type: 'preferences', preferences: { ...prefs, pet } })} />
        <div className="settings-group pet-controls"><div className="setting-row"><div className="setting-icon"><Icon name="screen" /></div><div className="setting-copy"><h3>Desktop companion</h3></div><Toggle label="Desktop companion" checked={prefs.petVisible} disabled={savingPreferences} change={petVisible => void act({ type: 'preferences', preferences: { ...prefs, petVisible } })} /></div><div className="setting-row"><div className="setting-icon"><Icon name="moon" /></div><div className="setting-copy"><h3>Quiet movements</h3></div><Toggle label="Quiet movements" checked={prefs.reduceMotion} disabled={savingPreferences} change={reduceMotion => void act({ type: 'preferences', preferences: { ...prefs, reduceMotion } })} /></div></div>
      </>}
      {page === 'security' && <>
        <header className="page-header"><h1>Security</h1></header>
        <div className="settings-group"><div className="setting-row"><div className="setting-icon"><Icon name="shield" /></div><div className="setting-copy"><h3>Pause SSH launches</h3><p>Block new launches from the {launchSurfaces}. Existing sessions keep running.</p></div><Toggle label="Pause SSH launches" checked={view.launchLocked} change={locked => void act({ type: 'launchLock', locked })} /></div></div>
        <h2 className="group-label">PROTECTIONS ON EVERY LAUNCH</h2><ul className="protection-list">{view.securitySummary.map(text => <li key={text}><Icon name="check" size={16} />{text}</li>)}</ul>
        <div className="info-card"><div><Icon name="info" /><h3>Your SSH config is trusted.</h3></div><p>OpenSSH still reads your local config, including proxy commands. Pausing launches is a convenience lock; it is not a password or account lock.</p></div>
        <div className="section-heading history-heading"><h2>Local activity</h2><span className="soft-label">Up to 200 events</span></div><p className="history-note">Requests and launch outcomes. No commands, addresses, passwords, or key contents. This local history is not tamper-proof.</p>
        <ol className="activity-list">{view.activity.map(event => <li key={event.id}><div><strong>{event.action}</strong><span>{event.outcome}</span></div><time dateTime={new Date(event.timestamp).toISOString()}>{new Date(event.timestamp).toLocaleString()}</time></li>)}</ol>{!view.activity.length && <p className="history-note">Your activity will appear here.</p>}
      </>}
      {page === 'preferences' && <>
        <header className="page-header"><h1>Preferences</h1></header>
        <div className="settings-group"><div className="setting-row"><div className="setting-icon"><Icon name="login" /></div><div className="setting-copy"><h3>Start with {platformName}</h3></div><Toggle label={`Start with ${platformName}`} checked={view.startAtLogin} change={enabled => void act({ type: 'startAtLogin', enabled })} /></div>{isWindows && <div className="setting-row"><div className="setting-icon"><Icon name="terminal" /></div><div className="setting-copy"><h3>SSH terminal</h3></div><select className="terminal-select" aria-label="SSH terminal" value={prefs.terminalShell} disabled={savingPreferences} onChange={event => void act({ type: 'preferences', preferences: { ...prefs, terminalShell: event.target.value as 'powerShell' | 'commandPrompt' } })}><option value="powerShell">PowerShell</option><option value="commandPrompt">Command Prompt</option></select></div>}</div>
        <footer className="page-footer"><Icon name="info" size={14} /><span>Closing this window keeps your pet and {trayName.toLowerCase()} available.</span></footer>
      </>}
    </main></div>
    {draft && <ConnectionDialog initial={draft} close={() => setDraft(undefined)} saved={refresh} />}
    {message && <div className={`toast ${message.error ? 'error' : ''}`} role="status"><Icon name={message.error ? 'info' : 'check'} size={18} /><span>{message.text}</span></div>}
  </>;
}

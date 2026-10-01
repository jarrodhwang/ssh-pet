import { useEffect, useRef, useState, type FormEvent } from 'react';
import { Icon } from './art';
import { errorText, mainCommand } from './bridge';
import type { ConnectionDraft } from './generated/ConnectionDraft';

export function ConnectionDialog({ initial, close, saved }: { initial: ConnectionDraft; close: () => void; saved: () => Promise<void> }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [draft, setDraft] = useState(initial);
  const [mode, setMode] = useState<'fields' | 'import'>('fields');
  const [command, setCommand] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [removing, setRemoving] = useState(false);
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    dialog.current?.showModal(); dialog.current?.querySelector('input')?.focus();
    return () => { opener?.focus(); };
  }, []);
  const submit = async (event: FormEvent) => {
    event.preventDefault(); setBusy(true); setError('');
    try {
      if (removing) await mainCommand({ type: 'delete', id: initial.id });
      else if (mode === 'import') await mainCommand({ type: 'import', name: draft.name, command });
      else await mainCommand({ type: 'save', draft });
      await saved(); close();
    } catch (error) { setError(errorText(error)); } finally { setBusy(false); }
  };
  const input = (name: keyof ConnectionDraft, label: string, placeholder = '') => <label className={`field ${name === 'port' ? 'port-field' : ''}`}>{label}<input name={name} autoComplete="off" spellCheck={false} placeholder={placeholder} value={draft[name]} onChange={e => setDraft({ ...draft, [name]: e.target.value })} /></label>;
  return <dialog ref={dialog} className="connection-dialog" aria-labelledby="dialog-title" onCancel={close}>
    <form noValidate onSubmit={submit}>
      <div className="dialog-heading"><h2 id="dialog-title">{removing ? 'Remove connection?' : initial.id ? 'Edit connection' : 'Add a connection'}</h2><button type="button" className="icon-button" aria-label="Close dialog" onClick={close}><Icon name="close" /></button></div>
      {removing ? <p>Remove “{initial.name}” from Droplet? Your SSH keys and existing Terminal sessions will stay where they are.</p> : <>
        {!initial.id && <div className="segmented-control"><button type="button" className={mode === 'fields' ? 'active' : ''} onClick={() => setMode('fields')}>Connection details</button><button type="button" className={mode === 'import' ? 'active' : ''} onClick={() => setMode('import')}>Paste SSH command</button></div>}
        {input('name', 'Connection name', 'My studio')}
        {mode === 'fields' ? <>
          {input('host', 'Hostname or IP address', 'studio.local or an SSH alias')}
          <div className="field-pair">{input('username', 'Username (optional)', 'From SSH config')}{input('port', 'Port')}</div>
          {input('identityFile', 'SSH key path (optional)', '~/.ssh/id_ed25519')}
          <p className="field-hint">Blank uses your SSH config or agent.</p>
        </> : <label className="field">SSH command<textarea name="command" rows={4} spellCheck={false} value={command} onChange={e => setCommand(e.target.value)} placeholder="ssh -i ~/.ssh/id_ed25519 you@studio.local" /><span className="field-hint">Paste one plain ssh command with optional -i and -p. The command is imported as connection details.</span></label>}
      </>}
      <p className="form-error" role="alert">{error}</p>
      <div className="dialog-footer">
        {initial.id && !removing ? <button type="button" className="text-button danger" onClick={() => { setRemoving(true); setError(''); }}>Remove connection</button> : <span className="privacy-hint">Saved only on this device.</span>}
        <div><button type="button" className="button secondary" onClick={close}>Cancel</button><button type="submit" className={`button ${removing ? 'danger-button' : 'primary'}`} disabled={busy}>{busy ? 'Working…' : removing ? 'Remove connection' : initial.id ? 'Save changes' : mode === 'import' ? 'Import connection' : 'Add connection'}</button></div>
      </div>
    </form>
  </dialog>;
}

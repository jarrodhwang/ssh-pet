import { useCallback, useEffect, useRef, useState, type PointerEvent } from 'react';
import { Droplet } from './art';
import { onChanged, petCommand } from './bridge';
import type { PetView } from './generated/PetView';
import type { PetRequest } from './generated/PetRequest';

export function Pet() {
  const [view, setView] = useState<PetView>();
  const pointer = useRef<{ id: number; x: number; y: number; moving: boolean } | null>(null);
  const queue = useRef(Promise.resolve());
  const clickTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const lastClick = useRef(0);
  const lastMove = useRef(0);
  const refresh = useCallback(async () => { const reply = await petCommand({ type: 'view' }); if (reply.type === 'view') setView(reply.value); }, []);
  const send = (request: PetRequest) => {
    queue.current = queue.current.then(async () => { await petCommand(request); }).catch(() => {});
  };
  useEffect(() => {
    document.body.classList.add('pet-window');
    let disposed = false; let unlisten = () => {};
    void refresh().catch(() => {});
    void onChanged(() => void refresh().catch(() => {})).then(fn => { if (disposed) fn(); else unlisten = fn; });
    return () => { disposed = true; unlisten(); clearTimeout(clickTimer.current); document.body.classList.remove('pet-window'); };
  }, [refresh]);
  useEffect(() => { document.body.classList.toggle('reduce-motion', !!view?.reduceMotion); }, [view?.reduceMotion]);
  const down = (event: PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    pointer.current = { id: event.pointerId, x: event.screenX, y: event.screenY, moving: false };
  };
  const move = (event: PointerEvent<HTMLButtonElement>) => {
    const p = pointer.current;
    if (!p || p.id !== event.pointerId) return;
    if (!p.moving && Math.hypot(event.screenX - p.x, event.screenY - p.y) > 5) {
      p.moving = true; clearTimeout(clickTimer.current); send({ type: 'beginDrag', x: p.x, y: p.y });
    }
    if (p.moving && performance.now() - lastMove.current > 32) {
      lastMove.current = performance.now(); send({ type: 'drag', x: event.screenX, y: event.screenY });
    }
  };
  const up = (event: PointerEvent<HTMLButtonElement>) => {
    const p = pointer.current; pointer.current = null;
    if (!p) return;
    if (p.moving) { send({ type: 'drag', x: event.screenX, y: event.screenY }); send({ type: 'endDrag' }); lastClick.current = 0; return; }
    if (lastClick.current > 0 && performance.now() - lastClick.current < 300) {
      clearTimeout(clickTimer.current); lastClick.current = 0; send({ type: 'connectFavorite' });
    } else {
      lastClick.current = performance.now();
      clickTimer.current = setTimeout(() => send({ type: 'openLauncher' }), 300);
    }
  };
  return <div className={`desktop-pet mood-${view?.mood || 'idle'}`}><span className="pet-tooltip">{view?.tooltip || 'Droplet'}</span><button className="pet-button" aria-label="Droplet: open connections" onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={() => { if (pointer.current?.moving) send({ type: 'endDrag' }); pointer.current = null; }} onClick={event => { if (event.detail === 0) send({ type: 'openLauncher' }); }}><Droplet /></button><span className="pet-status" role="status">{view?.status}</span></div>;
}

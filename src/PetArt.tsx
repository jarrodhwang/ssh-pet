import { useEffect, useRef, useState } from 'react';
import { Droplet } from './art';
import { characterAssets, idleDelay, petActions, type CharacterPet, type PetActionId, type PetPreview } from './petAnimation';
import type { PetKind } from './generated/PetKind';

function Effects({ action }: { action: PetActionId }) {
  if (action === 'spark' || action === 'volt-tackle') return <svg className={`pet-effects electricity ${action}`} viewBox="0 0 280 280" aria-hidden="true">
    <g className="electric-a"><path d="m67 91-19 22 17-3-12 25M210 106l18 19-18-2 12 25M78 167l-17 12 16 3-10 15" /><path d="m176 55 14-11-2 15 14-8M103 60l-8-14-5 12-7-6" /></g>
    <g className="electric-b"><path d="m66 130-20 15 18 3-10 20M204 77l17 17-15 1 15 15M205 174l21 11-17 7 13 9" /><path d="m83 210-12 8 10 4M177 207l18 11-8 6" /></g>
    {action === 'volt-tackle' && <><ellipse className="charge-ring" cx="140" cy="144" rx="95" ry="94" /><g className="speed-lines"><path d="M16 109h42M5 143h49M21 178h36" /></g></>}
  </svg>;
  if (action === 'iron-tail') return <svg className="pet-effects iron-tail" viewBox="0 0 280 280" aria-hidden="true"><path className="tail-sweep" d="M192 106C260 128 239 220 123 222" /><path className="tail-sweep second" d="M198 97C273 121 252 229 131 235" /><g className="metal-stars"><path d="m222 148 5 12 13 5-13 5-5 12-5-12-13-5 13-5Z" /><path d="m179 224 3 8 9 3-9 3-3 8-3-8-9-3 9-3Z" /></g></svg>;
  if (action === 'bubbles') return <svg className="pet-effects bubbles" viewBox="0 0 280 280" aria-hidden="true">{[0, 1, 2, 3, 4].map(i => <circle key={i} className={`bubble bubble-${i}`} cx={[64, 215, 75, 196, 132][i]} cy={[180, 163, 115, 93, 57][i]} r={[10, 16, 7, 9, 6][i]} />)}</svg>;
  if (action === 'flame') return <svg className="pet-effects flame" viewBox="0 0 280 280" aria-hidden="true"><g transform="translate(-130 -45)"><path className="flame-outer" d="M208 196c-28-20-9-34-10-52 19 13 4 24 18 28 9-8 9-15 8-25 22 29 14 50-16 49Z" /><path className="flame-inner" d="M208 194c-12-12-1-19 1-29 12 14 15 24-1 29Z" /><g className="embers"><circle cx="203" cy="120" r="3" /><circle cx="230" cy="132" r="2" /><circle cx="188" cy="152" r="2" /></g></g></svg>;
  if (action === 'petals') return <svg className="pet-effects petals" viewBox="0 0 280 280" aria-hidden="true">{[0, 1, 2, 3, 4].map(i => <path key={i} className={`petal petal-${i}`} d={`M${[60, 200, 85, 220, 155][i]} ${[85, 63, 150, 164, 40][i]}q-16 4-7 16 14-1 7-16`} />)}</svg>;
  if (action === 'sing') return <svg className="pet-effects music" viewBox="0 0 280 280" aria-hidden="true"><g className="note note-a"><path d="M63 123V87l20-5v35M63 94l20-5" /><ellipse cx="57" cy="123" rx="7" ry="5" /><ellipse cx="77" cy="117" rx="7" ry="5" /></g><g className="note note-b"><path d="M213 100V63l15 5v12" /><ellipse cx="207" cy="100" rx="7" ry="5" /></g><g className="note note-c"><path d="M191 192v-29l15-4" /><ellipse cx="185" cy="192" rx="7" ry="5" /></g></svg>;
  return null;
}

function Character({ pet, animate, reduceMotion, paused, preview }: {
  pet: CharacterPet; animate: boolean; reduceMotion: boolean; paused: boolean; preview?: PetPreview;
}) {
  const asset = characterAssets[pet];
  const actions = petActions[pet];
  const [visible, setVisible] = useState(() => !document.hidden);
  const [systemQuiet, setSystemQuiet] = useState(() => matchMedia('(prefers-reduced-motion: reduce)').matches);
  const [active, setActive] = useState<{ id: PetActionId; serial: number }>();
  const [failed, setFailed] = useState<string[]>([]);
  const sequence = useRef(0);
  const nextAction = useRef(0);
  const moving = animate && !systemQuiet && visible && !paused;
  useEffect(() => setActive(undefined), [reduceMotion]);
  useEffect(() => {
    if (!animate) return;
    const media = matchMedia('(prefers-reduced-motion: reduce)');
    const visibility = () => setVisible(!document.hidden);
    const motion = () => setSystemQuiet(media.matches);
    document.addEventListener('visibilitychange', visibility);
    media.addEventListener('change', motion);
    return () => { document.removeEventListener('visibilitychange', visibility); media.removeEventListener('change', motion); };
  }, [animate]);
  useEffect(() => {
    if (!moving) { setActive(undefined); return; }
    const timer = active
      ? setTimeout(() => setActive(undefined), actions.find(a => a.id === active.id)!.duration)
      : setTimeout(() => {
        const action = actions[nextAction.current++ % actions.length];
        setActive({ id: action.id, serial: ++sequence.current });
      }, idleDelay(reduceMotion));
    return () => clearTimeout(timer);
  }, [moving, active, actions, reduceMotion]);
  useEffect(() => {
    if (moving && preview && actions.some(a => a.id === preview.action)) {
      setActive({ id: preview.action, serial: ++sequence.current });
    }
  }, [moving, preview, actions]);
  const action = moving && active ? actions.find(a => a.id === active.id) : undefined;
  const clip = action?.clip;
  const wanted = moving
    ? (action && asset.moveAnimation ? asset.moveAnimation : (reduceMotion ? asset.quietAnimation : asset.animation)) ?? asset.poster
    : asset.poster;
  const image = failed.includes(wanted) ? asset.poster : wanted;
  const fail = (src: string) => setFailed(previous => previous.includes(src) ? previous : [...previous, src]);
  return <span className="droplet-art character-art" data-pet={pet} data-action={action?.id ?? 'idle'} data-motion={moving ? (reduceMotion ? 'gentle' : 'playing') : 'still'} data-asset-error={failed.length ? 'true' : undefined} aria-hidden="true">
    <span className="character-floor" />
    <span key={`${pet}-${active?.serial ?? 'idle'}`} className={`pet-motion ${!asset.animation ? 'render-idle' : ''} ${action && !clip ? `action-${action.id}` : ''} ${clip && !failed.includes(clip) ? 'behind-film' : ''}`}>
      {failed.includes(asset.poster) ? <span className="pet-art-unavailable">{asset.name}</span> : <img className="character-image" src={image} alt="" draggable={false} decoding="async" onError={() => fail(image)} />}
    </span>
    {clip && !failed.includes(clip) && <span className={`pet-film ${action?.id === 'happy-dance' ? 'transparent-clip' : ''}`} key={`${clip}-${active?.serial}`}><img src={clip} alt="" draggable={false} onError={() => fail(clip)} /></span>}
    {action && <Effects action={action.id} />}
  </span>;
}

export function PetArt({ pet = 'droplet', animated = false, reduceMotion = false, paused = false, preview }: {
  pet?: PetKind; animated?: boolean; reduceMotion?: boolean; paused?: boolean; preview?: PetPreview;
}) {
  if (pet === 'droplet') return <Droplet />;
  // Keying resets pending moves and failed assets when the selected pet changes.
  return <Character key={pet} pet={pet} animate={animated} reduceMotion={reduceMotion} paused={paused} preview={preview} />;
}

import { useEffect, useState } from 'react';
import { Icon } from './art';
import { PetArt } from './PetArt';
import { petActions, type PetPreview } from './petAnimation';
import type { PetKind } from './generated/PetKind';
import type { PetTheme } from './generated/PetTheme';
import type { PetThemeView } from './generated/PetThemeView';
import type { PetView } from './generated/PetView';

export function PetPicker({ themes, selected, select, disabled = false }: {
  themes: PetThemeView[]; selected: PetView; select: (pet: PetKind) => Promise<void>; disabled?: boolean;
}) {
  const [theme, setTheme] = useState<PetTheme>(selected.theme);
  const [pending, setPending] = useState<PetKind>();
  const [preview, setPreview] = useState<PetPreview>();
  const [systemQuiet, setSystemQuiet] = useState(() => matchMedia('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => setTheme(selected.theme), [selected.theme]);
  useEffect(() => setPreview(undefined), [selected.pet]);
  useEffect(() => {
    const media = matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => setSystemQuiet(media.matches);
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);
  const category = themes.find(item => item.id === theme) ?? themes[0];
  const actions = selected.pet === 'droplet' ? [] : petActions[selected.pet];
  const choose = async (pet: PetKind) => {
    if (disabled || pending || pet === selected.pet) return;
    setPending(pet);
    try { await select(pet); } finally { setPending(undefined); }
  };
  return <section className="pet-picker" aria-label="Pet selection">
    <div className="theme-list" role="group" aria-label="Pet themes">{themes.map(item =>
      <button key={item.id} className={`theme-button ${theme === item.id ? 'selected' : ''}`} aria-pressed={theme === item.id} onClick={() => setTheme(item.id)}>
        <PetArt pet={item.pets[0].id} /><span>{item.name}</span>
      </button>
    )}</div>
    <div className="pet-picker-body">
      <div className="selected-pet" aria-label="Current desktop pet">
        <div className="pet-stage"><PetArt pet={selected.pet} animated reduceMotion={selected.reduceMotion} preview={preview} /></div>
        <h2>{selected.name}</h2><span className="selected-pet-status"><i />{selected.status}</span>
        {!!actions.length && <div className="pet-action-list" role="group" aria-label="Preview pet animations">{actions.map(action =>
          <button key={action.id} disabled={systemQuiet} title={systemQuiet ? 'Motion is reduced in system settings' : `Preview ${action.name}`} aria-label={`Preview ${action.name}`} onClick={() => setPreview({ action: action.id, nonce: (preview?.nonce ?? 0) + 1 })}><span aria-hidden="true">▷</span>{action.name}</button>
        )}</div>}
        <span className="pet-gestures">Click to open · Double-click to connect · Drag to move</span>
      </div>
      <div className="pet-options"><div className="section-heading"><h2>{category.name}</h2><span className="soft-label">{category.pets.length} {category.pets.length === 1 ? 'pet' : 'pets'}</span></div>
        <div className="pet-grid" role="group" aria-label={`${category.name} pets`} aria-busy={disabled || !!pending}>{category.pets.map(pet =>
          <button key={pet.id} className={`pet-option ${selected.pet === pet.id ? 'selected' : ''}`} aria-label={`Select ${pet.name}`} aria-pressed={selected.pet === pet.id} disabled={disabled || !!pending} onClick={() => void choose(pet.id)}>
            <PetArt pet={pet.id} /><span>{pet.name}</span>{selected.pet === pet.id && <span className="pet-selection-mark"><Icon name="check" size={14} /></span>}{pending === pet.id && <span className="pet-selection-mark spinner" />}
          </button>
        )}</div>
      </div>
    </div>
  </section>;
}

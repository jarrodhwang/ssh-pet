import type { PetKind } from './generated/PetKind';

export type CharacterPet = Exclude<PetKind, 'droplet'>;
export type PetActionId = 'spark' | 'volt-tackle' | 'iron-tail' | 'cheese' | 'strawberry' | 'snack' | 'happy-dance' | 'flutter' | 'twirl' | 'petals' | 'flame' | 'bubbles' | 'sing' | 'pounce';
export type PetPreview = { action: PetActionId; nonce: number };
export type PetAction = { id: PetActionId; name: string; duration: number; clip?: string };

function character(pet: CharacterPet, name: string, animated = true) {
  return { name, poster: `/pets/${pet}-poster.webp`, animation: animated ? `/pets/${pet}.webp` : undefined,
    quietAnimation: animated ? `/pets/${pet}-quiet.webp` : undefined, moveAnimation: undefined as string | undefined };
}
export const characterAssets: Record<CharacterPet, ReturnType<typeof character>> = {
  remy: character('remy', 'Remy', false), emile: character('emile', 'Emile', false),
  pikachu: { ...character('pikachu', 'Pikachu'), poster: '/pets/pikachu-idle-poster.webp', animation: '/pets/pikachu-idle.webp', moveAnimation: '/pets/pikachu.webp' }, eevee: character('eevee', 'Eevee'),
  bulbasaur: character('bulbasaur', 'Bulbasaur'), charmander: character('charmander', 'Charmander'),
  squirtle: character('squirtle', 'Squirtle'), jigglypuff: character('jigglypuff', 'Jigglypuff'),
  snoopy: character('snoopy', 'Snoopy'), woodstock: character('woodstock', 'Woodstock'), belle: character('belle', 'Belle'),
};

export function idleDelay(quiet: boolean) {
  return quiet ? 90000 + Math.random() * 60000 : 40000 + Math.random() * 30000;
}

// Film/sticker clips preserve their original character frames. Transform/VFX moves
// add timing, travel and particles to that source art; they don't redraw the pet.
export const petActions: Record<CharacterPet, readonly PetAction[]> = {
  pikachu: [{ id: 'spark', name: 'Electricity', duration: 3200 }, { id: 'volt-tackle', name: 'Volt Tackle', duration: 3600 }, { id: 'iron-tail', name: 'Iron Tail', duration: 3000 }],
  remy: [{ id: 'cheese', name: 'Cheese', duration: 5600, clip: '/pets/remy-cheese.webp' }, { id: 'strawberry', name: 'Strawberry', duration: 5600, clip: '/pets/remy-strawberry.webp' }],
  emile: [{ id: 'snack', name: 'Snack', duration: 6000, clip: '/pets/emile-snack.webp' }],
  eevee: [{ id: 'pounce', name: 'Pounce', duration: 3200 }],
  bulbasaur: [{ id: 'petals', name: 'Petals', duration: 4000 }],
  charmander: [{ id: 'flame', name: 'Ember', duration: 3600 }],
  squirtle: [{ id: 'bubbles', name: 'Bubbles', duration: 4200 }],
  jigglypuff: [{ id: 'sing', name: 'Sing', duration: 4500 }],
  snoopy: [{ id: 'happy-dance', name: 'Happy dance', duration: 7600, clip: '/pets/snoopy-dance.webp' }],
  woodstock: [{ id: 'flutter', name: 'Flutter', duration: 3500 }],
  belle: [{ id: 'twirl', name: 'Twirl', duration: 3500 }],
};

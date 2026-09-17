import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { PhysicalPosition } from '@tauri-apps/api/dpi';

export interface Connection {
  id: string;
  name: string;
  host: string;
  username: string;
  port: number;
  identityFile: string;
}

export interface Config {
  version: number;
  connections: Connection[];
  favoriteId: string | null;
  petVisible: boolean;
  reduceMotion: boolean;
  petPosition: { x: number; y: number } | null;
}

export interface Snapshot {
  config: Config;
  startAtLogin: boolean;
  platform: string;
  notice: string | null;
  loadError: string | null;
}

export const native = isTauri();
const preview: Snapshot = {
  config: {
    version: 1,
    connections: [{ id: 'preview-studio', name: 'Zbook Studio', host: 'studio.local', username: 'you', port: 22, identityFile: '~/.ssh/id_ed25519' }],
    favoriteId: 'preview-studio', petVisible: true, reduceMotion: false, petPosition: null,
  },
  startAtLogin: false, platform: 'macos', notice: null, loadError: null,
};

export async function api<T = void>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (native) return invoke<T>(command, args);
  // A clearly labelled browser preview: local sample data, never a simulated live SSH session.
  if (command === 'get_snapshot') return structuredClone(preview) as T;
  if (command === 'connect') throw new Error('Open the Droplet desktop app to start an SSH session. This is a browser preview.');
  if (command === 'save_connection') {
    const connection = args.connection as Connection;
    if (!connection.id) {
      connection.id = crypto.randomUUID();
      preview.config.connections.push(connection);
      preview.config.favoriteId ??= connection.id;
    } else {
      preview.config.connections = preview.config.connections.map(item => item.id === connection.id ? connection : item);
    }
  } else if (command === 'delete_connection') {
    preview.config.connections = preview.config.connections.filter(item => item.id !== args.id);
    if (preview.config.favoriteId === args.id) preview.config.favoriteId = preview.config.connections[0]?.id ?? null;
  } else if (command === 'set_favorite') preview.config.favoriteId = args.id as string;
  else if (command === 'set_preferences') {
    preview.config.petVisible = args.petVisible as boolean;
    preview.config.reduceMotion = args.reduceMotion as boolean;
  } else if (command === 'set_start_at_login') {
    throw new Error('Launch at login is available in the installed desktop app.');
  } else if (command === 'import_command') {
    throw new Error('Import your command in the Droplet desktop app. The browser preview uses sample connections.');
  } else if (command === 'show_launcher') window.location.search = '';
  return undefined as T;
}

export async function on<T>(event: string, callback: (payload: T) => void): Promise<() => void> {
  return native ? listen<T>(event, event => callback(event.payload)) : () => {};
}

export async function drag(): Promise<void> {
  if (native) await getCurrentWindow().startDragging();
}

export async function preparePetMove(screenX: number, screenY: number): Promise<(x: number, y: number) => Promise<void>> {
  if (!native) return async () => {};
  const window = getCurrentWindow();
  const [origin, scale] = await Promise.all([window.outerPosition(), window.scaleFactor()]);
  // Screen coordinates stay stable when the window itself moves. Pointer capture
  // preserves click/double-click semantics, unlike starting an AppKit drag on mouse-down.
  return (x, y) => window.setPosition(new PhysicalPosition(
    Math.round(origin.x + (x - screenX) * scale),
    Math.round(origin.y + (y - screenY) * scale),
  ));
}

export async function trackPetPosition(): Promise<void> {
  if (!native) return;
  let timeout: ReturnType<typeof setTimeout>;
  await getCurrentWindow().onMoved(() => {
    clearTimeout(timeout);
    timeout = setTimeout(() => { void api('save_pet_position').catch(console.error); }, 400);
  });
}

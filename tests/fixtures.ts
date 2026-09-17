import { test as base, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { resolve } from 'node:path';

type IpcRequest = { surface: string; request: { type: string } };
export const test = base.extend<{ rustCore: void; ipcRequests: IpcRequest[] }>({
  ipcRequests: async ({}, use) => { await use([]); },
  rustCore: [async ({ page, ipcRequests }, use) => {
    const executable = resolve('src-tauri/target/debug/droplet-cli');
    const child = spawn(executable, ['test-ipc'], { stdio: ['pipe', 'pipe', 'pipe'] });
    const replies: { resolve: (value: unknown) => void; reject: (reason: Error) => void }[] = [];
    let errors = '';
    child.stderr.on('data', data => { errors += String(data); });
    const lines = createInterface({ input: child.stdout });
    lines.on('line', line => { const pending = replies.shift(); if (!pending) return; try { pending.resolve(JSON.parse(line)); } catch (error) { pending.reject(error as Error); } });
    child.on('error', error => { for (const pending of replies.splice(0)) pending.reject(error); });
    child.on('exit', () => { for (const pending of replies.splice(0)) pending.reject(new Error(`Rust test process exited: ${errors}`)); });
    await page.exposeFunction('__DROPLET_TEST_INVOKE__', (surface: string, request: unknown) => new Promise((resolve, reject) => {
      ipcRequests.push({ surface, request: request as { type: string } });
      replies.push({ resolve, reject }); child.stdin.write(JSON.stringify({ surface, request }) + '\n');
    }));
    try { await use(); } finally { lines.close(); child.stdin.end(); child.kill(); }
  }, { auto: true }],
});
export { expect };

# herdr-app 0.1.16: herdr.d.ts excerpts

Source: `~/.config/herdr-app/herdr.d.ts`, which herdr-app 0.1.16 rewrites on every start (AGENTS.md:44-47). Each block is the raw output of the command shown, run on 2026-11-22.

~~~~text
$ wc -l herdr.d.ts
     480 herdr.d.ts
~~~~

~~~~text
$ grep -n 'export function run\|export function commands' herdr.d.ts
91:export function run(name: string, args?: Record<string, unknown>): null;
282:export function commands(): { name: string; args: { name: string; required: boolean }[]; about: string }[];
~~~~


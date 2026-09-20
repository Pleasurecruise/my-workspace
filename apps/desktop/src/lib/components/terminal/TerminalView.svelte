<script lang="ts">
	import "@xterm/xterm/css/xterm.css";
	import { Channel, invoke } from "@tauri-apps/api/core";
	import { onMount, tick, untrack } from "svelte";
	import { Cable, RotateCcw, Square } from "@lucide/svelte";
	import type { Terminal } from "@xterm/xterm";
	import type { FitAddon } from "@xterm/addon-fit";
	import type { CommandResponse, TerminalTarget, TerminalConnection, TerminalOutput } from "../../consumer";

	let { target, active, locked }: { target: TerminalTarget; active: boolean; locked: boolean } = $props();
	const device = $derived(target.kind === "ssh" ? target.device : null);
	const usernameKey = untrack(() => device === null ? null : `vesper.ssh.username.${device.id}`);
	let username = $state(
		untrack(() => {
			if (device === null || usernameKey === null) return "";
			const remembered = localStorage.getItem(usernameKey);
			return remembered === null ? device.username : remembered;
		}),
	);
	let phase = $state<"idle" | "starting" | "running" | "closed">("idle");
	let error = $state<string | null>(null);
	let host = $state<HTMLDivElement | null>(null);
	let terminal = $state.raw<Terminal | null>(null);
	let fit: FitAddon | null = null;
	let sessionId: string | null = null;
	let generation = 0;
	let disposed = false;
	let inputQueue = Promise.resolve();
	let queuedBytes = 0;
	let resizeTimer: ReturnType<typeof setTimeout> | null = null;
	let lastSize = "";

	async function disconnect() {
		const version = ++generation;
		const id = sessionId;
		sessionId = null;
		phase = "closed";
		if (id !== null) {
			try {
				const response = await invoke<CommandResponse<null>>("disconnect_terminal", { sessionId: id });
				if (!disposed && version === generation && response.status === "failed")
					error = response.message;
			} catch {
				if (!disposed && version === generation)
					error = "Could not confirm that the terminal process stopped.";
			}
		}
	}

	async function connect() {
		if (terminal === null || locked || disposed || phase === "starting" || phase === "running")
			return;
		const login = username.trim();
		if (device !== null && login === "") {
			error = "Enter the remote device's login username.";
			return;
		}
		if (usernameKey !== null && /^(?!-)[A-Za-z0-9._-]{1,64}$/.test(login)) localStorage.setItem(usernameKey, login);
		await disconnect();
		if (locked || disposed) return;
		const version = ++generation;
		const id = crypto.randomUUID();
		sessionId = id;
		phase = "starting";
		error = null;
		terminal.reset();
		fit?.fit();
		const output = new Channel<TerminalOutput>();
		output.onmessage = (message) => {
			if (disposed || generation !== version || terminal === null) return;
			if (message.kind === "data") {
				terminal.write(new Uint8Array(message.bytes), () => {
					if (!disposed && generation === version)
						void invoke("acknowledge_terminal", { sessionId: id, bytes: message.bytes.length }).catch(
							() => {},
						);
				});
			} else if (message.kind === "exit") {
				sessionId = null;
				phase = "closed";
				terminal.write(
					`\r\n[Terminal session ended${message.code === null ? "" : ` · exit ${message.code}`} ]\r\n`,
				);
			} else if (sessionId === id) {
				error = message.message;
				sessionId = null;
				phase = "closed";
			}
		};
		try {
			const connection: TerminalConnection = device === null
				? { kind: "local" }
				: { kind: "ssh", deviceId: device.id, username: login };
			const response = await invoke<CommandResponse<null>>("connect_terminal", {
				request: { sessionId: id, target: connection, cols: terminal.cols, rows: terminal.rows },
				output,
			});
			if (disposed || version !== generation) {
				await invoke("disconnect_terminal", { sessionId: id });
				return;
			}
			if (response.status === "failed" && sessionId === id) {
				error = response.message;
				sessionId = null;
				phase = "closed";
			} else if (sessionId === id) {
				phase = "running";
				if (active) terminal.focus();
			}
		} catch {
			if (!disposed && version === generation && sessionId === id) {
				error = device === null ? "Could not start the local terminal. Try reconnecting." : "Could not start SSH. Refresh Tailscale and reconnect.";
				sessionId = null;
				phase = "closed";
			}
			void invoke("disconnect_terminal", { sessionId: id }).catch(() => {});
		}
	}

	function send(bytes: Uint8Array) {
		const id = sessionId;
		const version = generation;
		if (id === null || locked) return;
		if (bytes.length > 64 * 1024 || queuedBytes + bytes.length > 128 * 1024) {
			error = "Terminal input is busy or the paste exceeds 64 KB. Wait and try a smaller paste.";
			return;
		}
		queuedBytes += bytes.length;
		inputQueue = inputQueue.then(async () => {
			try {
				for (let offset = 0; offset < bytes.length; offset += 4096) {
					if (disposed || locked || version !== generation || sessionId !== id) return;
					const response = await invoke<CommandResponse<null>>("write_terminal", {
						sessionId: id,
						bytes: Array.from(bytes.subarray(offset, offset + 4096)),
					});
					if (response.status === "failed") {
						if (version === generation && sessionId === id) error = response.message;
						return;
					}
				}
			} catch {
				if (!disposed && version === generation && sessionId === id)
					error = "Could not send terminal input.";
			} finally {
				queuedBytes -= bytes.length;
			}
		});
	}

	function recordActivity(event: Event) {
		if (!event.isTrusted || disposed || locked || sessionId === null) return;
		void invoke("record_terminal_activity", { sessionId }).catch(() => {});
	}

	function resize() {
		if (disposed || !active || host === null || host.clientWidth === 0 || terminal === null) return;
		fit?.fit();
		const dimensions = `${terminal.cols}:${terminal.rows}`;
		if (sessionId !== null && dimensions !== lastSize) {
			lastSize = dimensions;
			const id = sessionId;
			void invoke<CommandResponse<null>>("resize_terminal", {
				sessionId,
				cols: terminal.cols,
				rows: terminal.rows,
			})
				.then((response) => {
					if (!disposed && sessionId === id && response.status === "failed" && phase === "running")
						error = response.message;
				})
				.catch(() => {
					if (!disposed && sessionId === id) error = "Could not resize the terminal.";
				});
		}
	}

	$effect(() => {
		if (locked && terminal !== null) void disconnect();
	});
	$effect(() => {
		if (active && terminal !== null)
			void tick().then(() => {
				resize();
				if (!locked) terminal?.focus();
			});
	});

	onMount(() => {
		let observer: ResizeObserver | null = null;
		const inputHost = host;
		for (const event of ["keydown", "paste", "compositionend", "pointerdown", "wheel"])
			inputHost?.addEventListener(event, recordActivity, true);
		void Promise.all([import("@xterm/xterm"), import("@xterm/addon-fit")])
			.then(([{ Terminal }, { FitAddon }]) => {
				if (disposed || host === null) return;
				const styles = getComputedStyle(host);
				const readColor = (role: string) =>
					styles.getPropertyValue(`--color-terminal-${role}`).trim();
				terminal = new Terminal({
					fontFamily: styles.getPropertyValue("--font-mono").trim(),
					fontSize: 13,
					lineHeight: 1.25,
					cursorBlink: !matchMedia("(prefers-reduced-motion: reduce)").matches,
					scrollback: 5000,
					screenReaderMode: true,
					convertEol: false,
					linkHandler: { activate() {} },
					theme: {
						background: readColor("background"),
						foreground: readColor("foreground"),
						cursor: readColor("foreground"),
						selectionBackground: readColor("selection"),
						black: readColor("black"),
						red: readColor("red"),
						green: readColor("green"),
						yellow: readColor("yellow"),
						blue: readColor("blue"),
						magenta: readColor("magenta"),
						cyan: readColor("cyan"),
						white: readColor("foreground"),
						brightBlack: readColor("muted"),
						brightRed: readColor("red"),
						brightGreen: readColor("green"),
						brightYellow: readColor("yellow"),
						brightBlue: readColor("blue"),
						brightMagenta: readColor("magenta"),
						brightCyan: readColor("cyan"),
						brightWhite: readColor("white"),
					},
				});
				fit = new FitAddon();
				terminal.loadAddon(fit);
				terminal.open(host);
				terminal.onData((data) => send(new TextEncoder().encode(data)));
				terminal.onBinary((data) =>
					send(Uint8Array.from(data, (character) => character.charCodeAt(0) & 255)),
				);
				observer = new ResizeObserver(() => {
					if (resizeTimer !== null) clearTimeout(resizeTimer);
					resizeTimer = setTimeout(resize, 80);
				});
				observer.observe(host);
				resize();
				void connect();
			})
			.catch(() => {
				if (!disposed) error = "The terminal renderer could not load. Reload Vesper and try again.";
			});
		return () => {
			for (const event of ["keydown", "paste", "compositionend", "pointerdown", "wheel"])
				inputHost?.removeEventListener(event, recordActivity, true);
			disposed = true;
			void disconnect();
			if (resizeTimer !== null) clearTimeout(resizeTimer);
			observer?.disconnect();
			terminal?.dispose();
			terminal = null;
		};
	});
</script>

<section class="terminal-view" hidden={!active} aria-label={device === null ? "Local terminal" : `${device.name} SSH terminal`}>
	<header class="page-header"><div><h1>{device === null ? "This device" : device.name || device.address}</h1><p class="page-description">{device === null ? "Local terminal" : `${device.dnsName || device.address} · ${device.os || "Remote device"}`}</p></div></header>
	<div class="terminal-frame">
		<div class="toolbar">
			<form onsubmit={(event) => { event.preventDefault(); void connect(); }}>
				{#if device !== null}<Cable size={14} /><label for={`ssh-user-${device.id}`}>Login as</label>
				<input id={`ssh-user-${device.id}`} bind:value={username} spellcheck="false" autocomplete="off" maxlength="64" disabled={phase === "starting" || phase === "running"} placeholder="Remote username" />{/if}
				{#if phase === "starting" || phase === "running"}<button type="button" onclick={() => void disconnect()}><Square size={11} /> Disconnect</button>{:else}<button type="submit" disabled={terminal === null || locked}><RotateCcw size={12} /> {phase === "idle" ? "Connect" : "Reconnect"}</button>{/if}
			</form>
			<span class="phase" role="status">{phase === "starting" ? (device === null ? "Starting shell…" : "Starting SSH…") : phase === "running" ? (device === null ? "Shell running" : "SSH running") : phase === "closed" ? "Disconnected" : "Ready"}</span>
		</div>
		{#if error}<p class="terminal-error" role="alert">{error}</p>{/if}
		<div class="terminal-host" bind:this={host}></div>
		<footer><span>{device === null ? "Local shell" : `Tailscale SSH · ${device.address}`}</span><span>{device === null ? "" : "Auto-disconnect after 5 min idle · "}Ctrl+C interrupts · exit disconnects</span></footer>
	</div>
	<p class="terminal-help">{device === null ? "Runs your current account’s default shell on this device. Closing or locking Vesper ends the session." : "Uses the remote account’s default shell. Enter host-key confirmation or authentication prompts in the terminal. Tailscale access rules still apply. Change the login name after disconnecting."}</p>
</section>

<style>
	.terminal-view { display: flex; flex: 1; flex-direction: column; min-width: 0; min-height: 0; background: var(--color-background); }
	.page-header { flex-shrink: 0; display: flex; align-items: center; gap: 8px; margin: 0; padding: 10px 14px; }
	.page-header > div { flex: 1; min-width: 0; }
	.page-header h1 { font-size: 1rem; }
	.terminal-view[hidden] { display: none; }
	.terminal-frame { display: flex; flex: 1; flex-direction: column; min-height: 0; overflow: hidden; border: 1px solid var(--color-border); border-radius: 0; background: var(--color-terminal-background); box-shadow: var(--shadow-sm); }
	.toolbar { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px; padding: 10px 14px; background: var(--color-muted); border-bottom: 1px solid var(--color-border); }
	form { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; color: var(--color-muted-foreground); font-size: 0.72rem; }
	input { width: 9rem; padding: 5px 8px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-foreground); font-family: var(--font-mono); font-size: 0.72rem; }
	form button { display: flex; align-items: center; gap: 5px; padding: 6px 9px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-foreground); font-size: 0.7rem; cursor: pointer; }
	button:disabled { opacity: 0.5; cursor: not-allowed; }
	button:focus-visible, input:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	.phase { color: var(--color-muted-foreground); font-size: 0.65rem; }
	.terminal-host { flex: 1; min-height: 0; overflow: hidden; padding: 12px; }
	.terminal-host :global(.xterm) { height: 100%; }
	.terminal-host :global(.xterm-viewport) { border-radius: var(--radius-sm); }
	.terminal-error { margin: 0; padding: 10px 14px; background: var(--color-muted); color: var(--color-error); font-size: 0.75rem; }
	footer { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 8px; padding: 8px 14px; border-top: 1px solid var(--color-terminal-selection); color: var(--color-terminal-muted); font-family: var(--font-mono); font-size: 0.6rem; }
	.terminal-help { flex-shrink: 0; margin: 0; padding: 6px 14px; color: var(--color-muted-foreground); font-size: 0.7rem; line-height: 1.6; }
</style>

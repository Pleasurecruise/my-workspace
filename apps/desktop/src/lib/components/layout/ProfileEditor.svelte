<script lang="ts">
	import { onMount, tick } from "svelte";
	let { compact }: { compact: boolean } = $props();
	const defaultProfileAvatar = new URL("../../../assets/pleasure1234-avatar.png", import.meta.url).href;
	const profileNameKey = "vesper.profile.name";
	const profileAvatarKey = "vesper.profile.avatar";
	let profileName = $state("Pleasure1234");
	let profileAvatar = $state(defaultProfileAvatar);
	let profileEditing = $state(false);
	let profileNameDraft = $state("Pleasure1234");
	let profileAvatarDraft = $state(defaultProfileAvatar);
	let profileError = $state<string | null>(null);
	let profileAvatarInput = $state<HTMLInputElement | null>(null);
	let profileNameInput = $state<HTMLInputElement | null>(null);
	let profilePopover = $state<HTMLDivElement | null>(null);
	async function toggleProfileEditor() {
		profileEditing = !profileEditing;
		profileNameDraft = profileName;
		profileAvatarDraft = profileAvatar;
		profileError = null;
		if (profileEditing) {
			await tick();
			const input = profileNameInput;
			if (input !== null) {
				input.focus();
				input.setSelectionRange(input.value.length, input.value.length);
			}
		}
	}

	function closeProfileEditorOnBlur(event: FocusEvent) {
		const next = event.relatedTarget;
		if (next instanceof Node && profilePopover?.contains(next)) return;
		window.setTimeout(() => {
			if (profilePopover?.contains(document.activeElement)) return;
			profileEditing = false;
			profileError = null;
		}, 0);
	}

	async function changeProfileAvatar(input: HTMLInputElement) {
		const files = input.files;
		input.value = "";
		if (files === null) return;
		const file = files.item(0);
		if (file === null) return;
		if (!file.type.startsWith("image/")) {
			profileError = "Choose an image file.";
			return;
		}
		let image: ImageBitmap | null = null;
		try {
			image = await createImageBitmap(file);
			const canvas = document.createElement("canvas");
			canvas.width = 256;
			canvas.height = 256;
			const context = canvas.getContext("2d");
			if (context === null) {
				profileError = "The image processor is unavailable.";
				return;
			}
			const sourceSize = Math.min(image.width, image.height);
			context.drawImage(
				image,
				(image.width - sourceSize) / 2,
				(image.height - sourceSize) / 2,
				sourceSize,
				sourceSize,
				0,
				0,
				canvas.width,
				canvas.height,
			);
			profileAvatarDraft = canvas.toDataURL("image/png");
			profileError = null;
		} catch {
			profileError = "This image could not be opened.";
		} finally {
			image?.close();
		}
	}

	function saveProfile(event: SubmitEvent) {
		event.preventDefault();
		const name = profileNameDraft.trim();
		if (name === "") {
			profileError = "Enter a username.";
			return;
		}
		try {
			localStorage.setItem(profileNameKey, name);
			localStorage.setItem(profileAvatarKey, profileAvatarDraft);
		} catch {
			profileError = "The profile could not be saved on this device.";
			return;
		}
		profileName = name;
		profileAvatar = profileAvatarDraft;
		profileEditing = false;
	}

	function resetProfileDraft() {
		profileNameDraft = "Pleasure1234";
		profileAvatarDraft = defaultProfileAvatar;
		profileError = null;
	}

	onMount(() => {
		profileName = localStorage.getItem(profileNameKey) ?? "Pleasure1234";
		profileAvatar = localStorage.getItem(profileAvatarKey) ?? defaultProfileAvatar;
	});
</script>

<div class="profile-popover-anchor" class:compact bind:this={profilePopover} onfocusout={closeProfileEditorOnBlur}>
	{#if profileEditing}
		<div class="profile-editor" role="dialog" aria-label="Edit local profile">
		<form onsubmit={saveProfile}>
			<div class="profile-editor-heading">
				<img src={profileAvatarDraft} alt="Profile preview" />
				<div><strong>Local profile</strong><span>Display only</span></div>
			</div>
			<label for="profile-name">Username</label>
			<input id="profile-name" bind:this={profileNameInput} maxlength="24" autocomplete="off" bind:value={profileNameDraft} />
			<input class="avatar-input" bind:this={profileAvatarInput} type="file" accept="image/*" onchange={(event) => void changeProfileAvatar(event.currentTarget)} />
			<div class="profile-editor-actions">
				<button type="button" onclick={() => profileAvatarInput?.click()}>Change photo</button>
				<button type="button" onclick={resetProfileDraft}>Reset</button>
				<button type="submit">Save</button>
			</div>
			{#if profileError !== null}<p role="alert">{profileError}</p>{/if}
		</form>
		</div>
	{/if}
	<button class="user-profile" type="button" onclick={toggleProfileEditor} aria-haspopup="dialog" aria-expanded={profileEditing} aria-label={`Edit local profile for ${profileName}`} title="Edit local profile">
		<img src={profileAvatar} alt="" />
		<span>{profileName}</span>
	</button>
</div>

<style>
	.user-profile {
		display: flex;
		flex: 1 1 auto;
		align-items: center;
		gap: 0.375rem;
		min-width: 0;
		height: 2rem;
		padding: 0 0.125rem;
		border: 0;
		border-radius: var(--radius-md);
		background: transparent;
		cursor: pointer;
		text-align: left;
	}

	.profile-popover-anchor {
		position: relative;
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
	}

	.user-profile:hover { background: var(--color-muted); }

	.user-profile img {
		width: 1.75rem;
		height: 1.75rem;
		flex: 0 0 auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		object-fit: cover;
	}

	.user-profile span {
		overflow: hidden;
		color: var(--color-foreground);
		font-size: 0.65rem;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.profile-editor {
		position: absolute;
		bottom: calc(100% + 0.75rem);
		left: 0;
		z-index: 30;
		display: grid;
		width: min(17rem, calc(100vw - 2rem));
		box-sizing: border-box;
		gap: 0.5rem;
		padding: 0.75rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-lg);
	}
	.profile-editor form { display: contents; }

	.profile-editor-heading { display: flex; align-items: center; gap: 0.625rem; }
	.profile-editor-heading img { width: 2.5rem; height: 2.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-full); object-fit: cover; }
	.profile-editor-heading div { display: grid; gap: 0.1rem; }
	.profile-editor-heading strong { font-size: 0.75rem; font-weight: 600; }
	.profile-editor-heading span,
	.profile-editor label { color: var(--color-muted-foreground); font-size: 0.65rem; }
	.profile-editor input:not(.avatar-input) { min-width: 0; height: 1.9rem; box-sizing: border-box; padding: 0 0.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); outline: none; background: var(--color-background); color: var(--color-foreground); font-size: 0.72rem; }
	.profile-editor input:not(.avatar-input):focus { border-color: var(--color-accent); box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 14%, transparent); }
	.avatar-input { display: none; }
	.profile-editor-actions { display: flex; gap: 0.3rem; }
	.profile-editor-actions button { height: 1.7rem; padding: 0 0.45rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: transparent; color: var(--color-foreground); cursor: pointer; font-size: 0.62rem; }
	.profile-editor-actions button:last-child { margin-left: auto; border-color: var(--color-accent); background: var(--color-accent); color: var(--color-accent-foreground); }
	.profile-editor p { margin: 0; color: var(--color-error); font-size: 0.62rem; }

	@media (min-width: 768px) {
		.compact .user-profile span { display: none; }
		.compact .user-profile {
			justify-content: center;
		}

		.compact .profile-editor {
			left: calc(100% + 0.75rem);
			bottom: 0;
		}
	}
</style>

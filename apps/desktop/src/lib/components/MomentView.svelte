<script lang="ts">
	import type { PhotoItem } from "../consumer";
	import R2Image from "./R2Image.svelte";

	let { photos, total }: { photos: PhotoItem[]; total: number } = $props();
	const dateFormatter = new Intl.DateTimeFormat("zh-CN", {
		year: "numeric",
		month: "2-digit",
		day: "2-digit",
	});
</script>

<section aria-label="Moment gallery">
	<div class="gallery-heading">
		<div>
			<p>Photo journal</p>
			<h2>Recent moments</h2>
		</div>
		<span>{total} photographs</span>
	</div>

	<div class="masonry">
		{#each photos as photo (photo.id)}
			<figure>
				<R2Image
					objectKey={photo.r2Key}
					thumbHash={photo.thumbHash}
					alt={photo.title}
					width={photo.width}
					height={photo.height}
				/>
				<figcaption>
					<strong>{photo.title}</strong>
					{#if photo.date}<span>{dateFormatter.format(new Date(photo.date))}</span>{/if}
					<div>
						{#each photo.tags as tag}<span>#{tag}</span>{/each}
					</div>
				</figcaption>
			</figure>
		{/each}
	</div>
</section>

<style>
	.gallery-heading {
		display: flex;
		align-items: end;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.gallery-heading p,
	.gallery-heading h2 {
		margin: 0;
	}

	.gallery-heading p {
		margin-bottom: 0.35rem;
		color: var(--color-accent);
		font-size: 0.72rem;
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.gallery-heading h2 {
		font-family: var(--font-serif);
		font-size: clamp(1.5rem, 3vw, 2.25rem);
		font-weight: 500;
		letter-spacing: -0.025em;
	}

	.gallery-heading > span {
		color: var(--color-muted-foreground);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		white-space: nowrap;
	}

	.masonry {
		columns: 3 14rem;
		column-gap: 0.65rem;
	}

	figure {
		position: relative;
		margin: 0 0 0.65rem;
		break-inside: avoid;
		overflow: hidden;
		border-radius: var(--radius-md);
	}

	figure :global(.progressive-image) {
		transition: scale var(--duration-slow);
	}

	figcaption {
		position: absolute;
		inset: auto 0 0;
		display: grid;
		gap: 0.3rem;
		padding: 3rem 1rem 0.9rem;
		background: linear-gradient(transparent, var(--color-image-scrim));
		color: var(--color-on-dark);
		opacity: 0;
		transition: opacity var(--duration-base);
	}

	figure:hover :global(.progressive-image) {
		scale: 1.025;
	}

	figure:hover figcaption,
	figure:focus-within figcaption {
		opacity: 1;
	}

	figcaption strong {
		font-size: 0.9rem;
	}

	figcaption > span,
	figcaption div {
		font-size: 0.7rem;
		opacity: 0.78;
	}

	figcaption div {
		display: flex;
		gap: 0.4rem;
	}

	@media (max-width: 700px) {
		.masonry {
			columns: 2 9rem;
		}
	}
</style>

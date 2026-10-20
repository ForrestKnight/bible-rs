import type { Attachment } from 'svelte/attachments';
import { reducedMotion, whileVisible } from '#lib/motion.svelte.js';
import { CUES, DURATION, SETTLED } from './script.js';
import { play, unlock } from './sound.js';

/**
 * Plays the demo while it's on screen: silently on a loop, or once with sound after the visitor
 * asks for it. With reduced motion it holds a still frame until the visitor presses play.
 */
export class Player {
	t = $state(0);
	playing = $state(true);
	sound = $state(false);
	#touched = $state(false);

	/** Reduced motion only stops the demo until the visitor starts it themselves. */
	#moving = $derived(!reducedMotion.current || this.#touched);

	time = $derived(this.#moving ? this.t : SETTLED);
	paused = $derived(!this.playing || !this.#moving);

	#advance(dt: number) {
		const from = this.t;
		const to = from + dt;
		if (this.sound) {
			for (const cue of CUES) if (cue.t > from && cue.t <= to) play(cue.sound, cue.variant);
		}
		if (to < DURATION) {
			this.t = to;
		} else if (this.sound) {
			this.t = DURATION;
			this.playing = false;
		} else {
			this.t = 0;
		}
	}

	observe: Attachment<HTMLElement> = (node) => {
		if (!this.playing || !this.#moving) return;
		return whileVisible(node, (dt) => this.#advance(dt));
	};

	playWithSound() {
		unlock();
		this.sound = true;
		this.#start(0);
	}

	toggle() {
		if (this.paused) this.#start(this.t >= DURATION ? 0 : this.t);
		else this.playing = false;
	}

	replay() {
		this.#start(0);
	}

	toggleSound() {
		if (!this.sound) unlock();
		this.sound = !this.sound;
	}

	seek(t: number) {
		this.#touched = true;
		this.t = t;
	}

	#start(t: number) {
		this.#touched = true;
		this.t = t;
		this.playing = true;
	}
}

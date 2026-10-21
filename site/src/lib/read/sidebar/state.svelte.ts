import type { StudyKind } from '../app.svelte';
import type { HighlightColor } from '../core';

export const STUDY_KINDS: StudyKind[] = ['bookmarks', 'highlights', 'notes'];

/** What the source list remembers while search takes its place (sidebar.rs `Sidebar`). */
export class SourceListState {
	/** The Old and New Testament groups. */
	testaments = $state([false, false]);
	/** Groups opened to reveal the book being read, which close again when it leaves them. */
	autoOpened = [false, false];
	/** The book whose chapters show. */
	expanded = $state<number | null>(null);
	/** The book last revealed (its testament opened and it scrolled into view). */
	revealed: number | null = null;
	/** Study collections listing their recent items in place, and those showing all of them. */
	studyOpen = $state([false, false, false]);
	studyAll = $state([false, false, false]);
	highlightFilter = $state<HighlightColor | null>(null);
	/** Type-to-filter over the books. */
	filtering = $state(false);
	filter = $state('');
	/** Bumped to focus the filter field. */
	filterFocus = $state(0);

	toggleStudy(kind: StudyKind) {
		const i = STUDY_KINDS.indexOf(kind);
		this.studyOpen[i] = !this.studyOpen[i];
	}

	startFilter(text: string) {
		this.filtering = true;
		this.filter = text;
		this.filterFocus++;
	}

	endFilter() {
		this.filtering = false;
		this.filter = '';
	}
}

/**
 * The Ark in three.js, built from objects/ark.json: every size comes from the spec's dims, so a
 * corrected number there moves the model too. Viewer.svelte loads this module (and three.js with
 * it) only once the page has painted, and only where WebGL works; no other page imports it.
 *
 * Coordinates are cubits: x along the ark's length, y up, z across its breadth, the floor at y = 0.
 */
import {
	ACESFilmicToneMapping,
	AdditiveBlending,
	BoxGeometry,
	CanvasTexture,
	Color,
	CylinderGeometry,
	DirectionalLight,
	DoubleSide,
	Fog,
	GridHelper,
	Group,
	HemisphereLight,
	type Material,
	Mesh,
	MeshPhysicalMaterial,
	MeshStandardMaterial,
	type Object3D,
	PCFShadowMap,
	PerspectiveCamera,
	PlaneGeometry,
	PMREMGenerator,
	PointLight,
	Raycaster,
	Scene,
	Sprite,
	SpriteMaterial,
	TorusGeometry,
	Vector2,
	Vector3,
	WebGLRenderer
} from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { buildCherub, DRAWN_HEIGHT, hammeredTexture } from './cherub';
import { chosen, type ObjectSpec, type Part } from './spec';

export interface SceneEvents {
	/** A part was tapped (or the empty floor: null). */
	pick(id: string | null): void;
	/** The assembly reached a step (its index in spec.assembly), or ended (null). */
	step(index: number | null): void;
	/** The first frame is on screen. */
	ready(): void;
}

export interface Hotspot {
	part: string;
	/** In CSS pixels from the stage's top left; null while the part is hidden or faces away. */
	at: { x: number; y: number } | null;
}

export interface ArkScene {
	setInside(on: boolean): void;
	setCloseUp(on: boolean): void;
	/** A part's debated option set to one of its choices (the rings' position). */
	setChoice(part: string, option: string, choice: string): void;
	select(id: string | null): void;
	/** Plays "Build from the text" from the start. */
	build(): void;
	/** Ends the assembly now, with everything in place. */
	finish(): void;
	readonly building: boolean;
	/** Where each part's hotspot sits on screen, after the latest frame. */
	hotspots(): Hotspot[];
	onHotspots(fn: () => void): void;
	dispose(): void;
}

const GOLD = 0xe6b84f;
const WOOD = 0x8a5a33;
const BACKDROP = 0x14110d;

/** How long each step of the assembly takes to land, and how long its verse stays as the caption. */
const LAND = 1.1;
const STEP = 2.1;

const ease = (x: number) => 1 - Math.pow(1 - Math.min(Math.max(x, 0), 1), 3);

/** A part's number from the spec, which check.mjs has made sure is there (its SCENE table). */
function dim(part: Part | undefined, key: string): number {
	const d = part?.dims[key];
	if (!d) throw new Error(`objects: ${part?.id ?? '?'} has no ${key}`);
	return d.value;
}

export function createScene(
	stage: HTMLElement,
	spec: ObjectSpec,
	{ reducedMotion, touchScroll, tight = false, events }: {
		reducedMotion: boolean;
		/** On a page that scrolls, a vertical swipe scrolls it; a sideways one turns the ark. */
		touchScroll: boolean;
		/** Frame the ark closer (the share image). */
		tight?: boolean;
		events: SceneEvents;
	}
): ArkScene {
	const part = (id: string) => spec.parts.find((p) => p.id === id);
	const chestSpec = part('chest');
	const L = dim(chestSpec, 'length');
	const B = dim(chestSpec, 'breadth');
	const H = dim(chestSpec, 'height');

	// ------------------------------------------------------------------ renderer, camera, light
	const renderer = new WebGLRenderer({ antialias: true, powerPreference: 'low-power' });
	renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
	renderer.shadowMap.enabled = true;
	renderer.shadowMap.type = PCFShadowMap;
	renderer.toneMapping = ACESFilmicToneMapping;
	renderer.toneMappingExposure = 1.05;
	renderer.domElement.className = 'gl';
	renderer.domElement.setAttribute('role', 'img');
	renderer.domElement.setAttribute('aria-label', `${spec.title} in 3D. Drag to turn it; the numbered spots and the list of parts open each part's verses.`);
	stage.prepend(renderer.domElement);

	const scene = new Scene();
	scene.background = new Color(BACKDROP);
	scene.fog = new Fog(BACKDROP, 9, 22);
	const pmrem = new PMREMGenerator(renderer);
	scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;

	const camera = new PerspectiveCamera(35, 1, 0.05, 100);
	const controls = new OrbitControls(camera, renderer.domElement);
	controls.target.set(0, 0.95, 0);
	controls.enableDamping = !reducedMotion;
	controls.enablePan = false;
	controls.minDistance = 2.2;
	controls.maxDistance = 14;
	controls.maxPolarAngle = Math.PI * 0.49;
	if (touchScroll) renderer.domElement.style.touchAction = 'pan-y';

	scene.add(new HemisphereLight(0xfff1d6, 0x2a2016, 0.6));
	const sun = new DirectionalLight(0xffe2b0, 2.2);
	sun.position.set(3, 6, 2.5);
	sun.castShadow = true;
	sun.shadow.mapSize.set(2048, 2048);
	Object.assign(sun.shadow.camera, { left: -4, right: 4, top: 4, bottom: -4 });
	scene.add(sun);

	const floor = new Mesh(new PlaneGeometry(40, 40), new MeshStandardMaterial({ color: 0x2a231b, roughness: 0.95 }));
	floor.rotation.x = -Math.PI / 2;
	floor.receiveShadow = true;
	scene.add(floor);
	// One cell is one cubit.
	const grid = new GridHelper(10, 10, 0x6b5a40, 0x3d3326);
	grid.position.y = 0.002;
	scene.add(grid);

	// ------------------------------------------------------------------ parts
	const gold = () => new MeshStandardMaterial({ color: GOLD, metalness: 1, roughness: 0.28 });
	const wood = () => new MeshStandardMaterial({ color: WOOD, metalness: 0, roughness: 0.8 });
	const stone = () => new MeshStandardMaterial({ color: 0x9c968b, roughness: 0.9 });

	const groups = new Map<string, Group>();
	const group = (id: string) => {
		const g = new Group();
		g.userData.part = id;
		groups.set(id, g);
		scene.add(g);
		return g;
	};
	const mesh = (geo: ConstructorParameters<typeof Mesh>[0], mat: Material, g: Group, pos = [0, 0, 0], rot = [0, 0, 0]) => {
		const m = new Mesh(geo, mat);
		m.position.set(pos[0], pos[1], pos[2]);
		m.rotation.set(rot[0], rot[1], rot[2]);
		m.castShadow = m.receiveShadow = true;
		g.add(m);
		return m;
	};

	// The chest (Exodus 25:10), acacia, then its gold overlay within and without (25:11): a thin
	// shell the gild step fades in over the wood.
	const chest = group('chest');
	const chestWood = mesh(new BoxGeometry(L, H, B), wood(), chest, [0, H / 2, 0]);
	const skin = dim(chestSpec, 'gold');
	const overlay = new Group();
	chest.add(overlay);
	const shell = mesh(new BoxGeometry(L + 2 * skin, H + skin, B + 2 * skin), gold(), overlay as Group, [0, H / 2, 0]);
	// Faint panel lines, so the gold reads as a worked object.
	for (const s of [-1, 1]) mesh(new BoxGeometry(L * 0.86, 0.012, 0.01), gold(), overlay as Group, [0, H * 0.18, s * (B / 2 + skin + 0.002)]);

	// The crown of gold round about (25:11), a rim at the top edge.
	const crownSpec = part('crown');
	const crown = group('crown');
	const cH = dim(crownSpec, 'height');
	const cT = dim(crownSpec, 'thickness');
	const cy = H - 0.02;
	mesh(new BoxGeometry(L + 2 * cT, cH, cT), gold(), crown, [0, cy, B / 2 + cT / 2]);
	mesh(new BoxGeometry(L + 2 * cT, cH, cT), gold(), crown, [0, cy, -B / 2 - cT / 2]);
	mesh(new BoxGeometry(cT, cH, B), gold(), crown, [L / 2 + cT / 2, cy, 0]);
	mesh(new BoxGeometry(cT, cH, B), gold(), crown, [-L / 2 - cT / 2, cy, 0]);

	// Four rings (25:12), two on each long side; how high is a debated option.
	const ringSpec = part('rings');
	const rings = group('rings');
	const ringR = dim(ringSpec, 'radius');
	const ringX = L / 2 - dim(ringSpec, 'inset');
	const ringZ = B / 2 + ringR + 0.02;
	const ringHeight = (choice?: string) => (ringSpec && chosen(ringSpec, 'position', choice)?.values.height?.value) ?? 0.32;
	const ringMeshes: Mesh[] = [];
	for (const sx of [-1, 1])
		for (const sz of [-1, 1])
			ringMeshes.push(
				mesh(new TorusGeometry(ringR, dim(ringSpec, 'thickness'), 12, 32), gold(), rings, [sx * ringX, 0, sz * ringZ], [0, Math.PI / 2, 0])
			);

	// The staves (25:13–15), acacia overlaid with gold, through the rings.
	const poleSpec = part('poles');
	const poles = group('poles');
	const poleMats: MeshStandardMaterial[] = [];
	const poleMeshes: Mesh[] = [];
	for (const sz of [-1, 1]) {
		const m = wood();
		poleMats.push(m);
		poleMeshes.push(
			mesh(new CylinderGeometry(dim(poleSpec, 'radius'), dim(poleSpec, 'radius'), dim(poleSpec, 'length'), 20), m, poles, [0, 0, sz * ringZ], [0, 0, Math.PI / 2])
		);
	}
	const setRingHeight = (y: number) => {
		ringMeshes.forEach((m) => (m.position.y = y));
		poleMeshes.forEach((m) => (m.position.y = y));
	};
	setRingHeight(ringHeight());

	// The testimony (25:16): two tables of stone (31:18), inside.
	const tabletSpec = part('testimony');
	const testimony = group('testimony');
	const tw = dim(tabletSpec, 'width');
	const th = dim(tabletSpec, 'height');
	for (const dx of [-0.55, 0.55]) mesh(new BoxGeometry(tw, th, dim(tabletSpec, 'thickness')), stone(), testimony, [dx * tw, th / 2 + 0.05, 0]);

	// The mercy seat (25:17, 21), pure gold, set above on the ark.
	const seatSpec = part('mercy-seat');
	const mercy = group('mercy-seat');
	const mT = dim(seatSpec, 'thickness');
	mesh(new BoxGeometry(dim(seatSpec, 'length'), mT, dim(seatSpec, 'breadth')), gold(), mercy, [0, H + mT / 2, 0]);

	// The cherubim (25:18–20), the fuller artistic figures the spec labels interpretive.
	const cherubSpec = part('cherubim');
	const cherubim = group('cherubim');
	const hammer = hammeredTexture();
	hammer.repeat.set(2, 2);
	const scale = dim(cherubSpec, 'height') / DRAWN_HEIGHT;
	const inset = dim(cherubSpec, 'inset');
	for (const s of [-1, 1]) {
		const g = new Group();
		g.position.set(s * (L / 2 - inset), H + mT, 0);
		// Local +x faces the centre of the seat, so the faces "look one to another".
		g.rotation.y = s > 0 ? Math.PI : 0;
		g.scale.setScalar(scale);
		const m = new MeshPhysicalMaterial({
			color: 0xe9bd55,
			metalness: 1,
			roughness: 0.34,
			bumpMap: hammer,
			bumpScale: 1.2,
			clearcoat: 0.25,
			clearcoatRoughness: 0.4
		});
		const feathers = m.clone();
		feathers.side = DoubleSide;
		feathers.roughness = 0.3;
		g.add(buildCherub(m, feathers));
		cherubim.add(g);
	}

	// "There I will meet with thee ... from between the two cherubims" (25:22): only a soft light.
	const presence = group('presence');
	const glowAt = new Vector3(0, H + mT + 0.5, 0);
	const glow = new PointLight(0xfff3c4, 0, 3, 2);
	glow.position.copy(glowAt);
	presence.add(glow);
	const haloMat = new SpriteMaterial({
		map: radial(),
		color: 0xfff1c0,
		transparent: true,
		opacity: 0,
		depthWrite: false,
		blending: AdditiveBlending
	});
	const halo = new Sprite(haloMat);
	halo.scale.set(1.3, 1.3, 1);
	halo.position.copy(glowAt);
	presence.add(halo);

	// Every material, by part, for fading parts in and for the selection's glow.
	const materials = (g: Object3D) => {
		const out: MeshStandardMaterial[] = [];
		g.traverse((o) => {
			const m = (o as Mesh).material as MeshStandardMaterial | undefined;
			if (m && 'emissive' in m) out.push(m);
		});
		return out;
	};

	// ------------------------------------------------------------------ hotspots
	// Each part's spot, with the way it faces: the spot shows on whichever candidate faces the
	// camera most, and hides when none does. No normal means it always shows.
	const hotspotAt: Record<string, { pos: Vector3; normal?: Vector3 }[]> = {
		chest: [
			{ pos: new Vector3(0, H * 0.55, B / 2 + skin), normal: new Vector3(0, 0, 1) },
			{ pos: new Vector3(0, H * 0.55, -B / 2 - skin), normal: new Vector3(0, 0, -1) },
			{ pos: new Vector3(L / 2 + skin, H * 0.55, 0), normal: new Vector3(1, 0, 0) },
			{ pos: new Vector3(-L / 2 - skin, H * 0.55, 0), normal: new Vector3(-1, 0, 0) }
		],
		crown: [
			{ pos: new Vector3(L * 0.3, cy, B / 2 + cT), normal: new Vector3(0, 0.3, 1) },
			{ pos: new Vector3(-L * 0.3, cy, -B / 2 - cT), normal: new Vector3(0, 0.3, -1) }
		],
		rings: [],
		poles: [],
		testimony: [{ pos: new Vector3(0, th + 0.12, 0) }],
		'mercy-seat': [{ pos: new Vector3(0, H + mT, B * 0.3), normal: new Vector3(0, 1, 0.4) }],
		cherubim: [{ pos: new Vector3(L / 2 - inset, H + mT + 0.62 * scale, 0) }],
		presence: [{ pos: glowAt }]
	};
	const placeRingSpots = () => {
		const y = ringMeshes[0].position.y;
		hotspotAt.rings = [
			{ pos: new Vector3(ringX, y + ringR, ringZ), normal: new Vector3(0, 0, 1) },
			{ pos: new Vector3(-ringX, y + ringR, -ringZ), normal: new Vector3(0, 0, -1) }
		];
		const end = dim(poleSpec, 'length') / 2 - 0.15;
		hotspotAt.poles = [
			{ pos: new Vector3(end, y, ringZ), normal: new Vector3(1, 0, 0.5) },
			{ pos: new Vector3(-end, y, -ringZ), normal: new Vector3(-1, 0, -0.5) }
		];
	};
	placeRingSpots();

	// ------------------------------------------------------------------ state
	let inside = false;
	let closeUp = false;
	let selected: string | null = null;
	let built = new Set(spec.parts.map((p) => p.id));
	/** Seconds into the assembly, or null when it isn't playing. */
	let t0: number | null = null;
	let lastStep: number | null = null;
	let disposed = false;
	let size = { w: 0, h: 0 };
	const listeners: (() => void)[] = [];

	/** The assembly's timeline: when each step starts. */
	const steps = spec.assembly.map((s, i) => ({ ...s, at: i * STEP }));
	const END = steps.length * STEP + 0.6;

	function setOpacity(g: Object3D, a: number) {
		for (const m of materials(g)) {
			const transparent = a < 0.999;
			if (m.transparent !== transparent) {
				m.transparent = transparent;
				m.needsUpdate = true;
			}
			m.opacity = a;
		}
		g.visible = a > 0.01;
	}

	function applyInside() {
		const ghost = inside;
		for (const [m, a] of [
			[shell.material as MeshStandardMaterial, 0.22],
			[chestWood.material as MeshStandardMaterial, 0.12]
		] as const) {
			m.transparent = ghost;
			m.opacity = ghost ? a : 1;
			m.depthWrite = !ghost;
			m.needsUpdate = true;
		}
		overlay.children.slice(1).forEach((o) => (o.visible = !ghost));
	}

	/** Every part where the assembly has it at `t` seconds (Infinity: all in place). */
	function pose(t: number) {
		const reveal = (id: string, k: number, slide = false) => {
			const g = groups.get(id)!;
			const home = 0;
			if (slide) g.position.x = reducedMotion ? home : home + (1 - k) * 4;
			else g.position.y = reducedMotion ? home : home + (1 - k) * 1.1;
			setOpacity(g, reducedMotion ? (k > 0 ? 1 : 0) : k);
		};
		built = new Set(spec.parts.filter((p) => !steps.some((s) => s.part === p.id)).map((p) => p.id));
		for (const s of steps) {
			const k = ease((t - s.at) / LAND);
			if (k > 0) built.add(s.part);
			if (s.effect === 'gild') {
				const gk = reducedMotion ? (k > 0 ? 1 : 0) : k;
				setOpacity(overlay, gk);
				continue;
			}
			if (s.part === 'presence') {
				const gk = reducedMotion ? (k > 0 ? 1 : 0) : k;
				glow.intensity = 2.5 * gk;
				haloMat.opacity = 0.75 * gk;
				presence.visible = gk > 0;
				continue;
			}
			if (s.part === 'chest') {
				// The wood only; the overlay has its own step.
				const g = groups.get('chest')!;
				g.position.y = reducedMotion ? 0 : (1 - k) * 1.1;
				setOpacity(chestWood, reducedMotion ? (k > 0 ? 1 : 0) : k);
				chestWood.visible = k > 0;
				continue;
			}
			reveal(s.part, k, s.part === 'poles');
		}
		// The staves turn from acacia to gold just after they arrive ("overlay them with gold").
		const poleStep = steps.find((s) => s.part === 'poles');
		const pk = poleStep ? ease((t - poleStep.at - LAND) / 0.6) : 1;
		for (const m of poleMats) {
			m.color.setHex(pk > 0.5 ? GOLD : WOOD);
			m.metalness = pk;
			m.roughness = 0.8 - 0.52 * pk;
		}
	}

	function highlight() {
		for (const [id, g] of groups) {
			for (const m of materials(g)) m.emissive.setHex(id === selected ? 0x3a2600 : 0);
		}
	}

	// ------------------------------------------------------------------ camera
	const wide = { pos: new Vector3(4.9, 3.1, 6.3), target: new Vector3(-0.3, 1.25, 0) };
	const near = { pos: new Vector3(1.9, 2.75, 2.6), target: new Vector3(0, 1.85, 0) };
	let fly: { from: Vector3; to: Vector3; fromT: Vector3; toT: Vector3; start: number } | null = null;

	/** Far enough back that the ark and its staves fit the stage at any aspect. */
	function fit(base: Vector3, target: Vector3, radius: number) {
		const vfov = (camera.fov * Math.PI) / 180;
		const hfov = 2 * Math.atan(Math.tan(vfov / 2) * camera.aspect);
		const d = radius / Math.sin(Math.min(vfov, hfov) / 2);
		return target.clone().add(base.clone().sub(target).normalize().multiplyScalar(Math.max(d, base.distanceTo(target))));
	}

	function flyTo(view: typeof wide, instant: boolean) {
		// The close-up frames the cherubim; the wide view the ark and its staves.
		const to = fit(view.pos, view.target, view === near ? 1.15 : tight ? 2.3 : 2.9);
		if (instant || reducedMotion) {
			camera.position.copy(to);
			controls.target.copy(view.target);
			fly = null;
		} else {
			fly = { from: camera.position.clone(), to, fromT: controls.target.clone(), toT: view.target.clone(), start: performance.now() };
		}
		wake();
	}

	function resize() {
		const w = stage.clientWidth;
		const h = stage.clientHeight;
		if (!w || !h || (w === size.w && h === size.h)) return;
		size = { w, h };
		renderer.setSize(w, h, false);
		camera.aspect = w / h;
		camera.updateProjectionMatrix();
		if (!closeUp && !fly) flyTo(wide, true);
		wake();
	}

	// ------------------------------------------------------------------ picking
	const ray = new Raycaster();
	const ptr = new Vector2();
	let down: [number, number] | null = null;
	const el = renderer.domElement;
	const onDown = (e: PointerEvent) => {
		down = [e.clientX, e.clientY];
		wake();
	};
	const onUp = (e: PointerEvent) => {
		if (!down || Math.hypot(e.clientX - down[0], e.clientY - down[1]) > 6) return;
		down = null;
		const r = el.getBoundingClientRect();
		ptr.set(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
		ray.setFromCamera(ptr, camera);
		const targets = [...groups.values()].filter((g) => g.visible);
		const hit = ray.intersectObjects(targets, true).find((h) => {
			if (!h.object.visible) return false;
			const m = (h.object as Mesh).material as Material | undefined;
			return !m || m.opacity > 0.3;
		});
		let o: Object3D | null = hit?.object ?? null;
		while (o && !o.userData.part) o = o.parent;
		events.pick((o?.userData.part as string | undefined) ?? null);
	};
	el.addEventListener('pointerdown', onDown);
	el.addEventListener('pointerup', onUp);
	controls.addEventListener('change', () => wake());

	// ------------------------------------------------------------------ loop
	let running = false;
	let idleSince = performance.now();
	let visible = true;
	let first = true;
	const projected = new Vector3();
	const toCamera = new Vector3();
	let spots: Hotspot[] = [];

	function measureSpots() {
		spots = spec.parts.map((p) => {
			const g = groups.get(p.id);
			if (!g?.visible || !built.has(p.id) || (p.id === 'testimony' && !inside)) return { part: p.id, at: null };
			let best: { pos: Vector3; score: number } | null = null;
			for (const c of hotspotAt[p.id] ?? []) {
				toCamera.copy(camera.position).sub(c.pos).normalize();
				const score = c.normal ? c.normal.clone().normalize().dot(toCamera) : 1;
				if (score > 0.15 && (!best || score > best.score)) best = { pos: c.pos, score };
			}
			if (!best) return { part: p.id, at: null };
			projected.copy(best.pos).project(camera);
			if (projected.z > 1) return { part: p.id, at: null };
			return { part: p.id, at: { x: ((projected.x + 1) / 2) * size.w, y: ((1 - projected.y) / 2) * size.h } };
		});
		listeners.forEach((fn) => fn());
	}

	function frame(now: number) {
		if (disposed) return;
		resize();
		let moving = false;
		if (t0 !== null) {
			const t = (now - t0) / 1000;
			pose(t);
			let idx: number | null = null;
			for (let i = 0; i < steps.length; i++) if (t >= steps[i].at) idx = i;
			if (t > END) {
				t0 = null;
				idx = null;
				pose(Infinity);
				applyInside();
			}
			if (idx !== lastStep) {
				lastStep = idx;
				events.step(idx);
			}
			moving = true;
		}
		if (fly) {
			const k = ease((now - fly.start) / 800);
			camera.position.lerpVectors(fly.from, fly.to, k);
			controls.target.lerpVectors(fly.fromT, fly.toT, k);
			if (k >= 1) fly = null;
			moving = true;
		}
		if (controls.update()) moving = true;
		renderer.render(scene, camera);
		measureSpots();
		if (first) {
			first = false;
			events.ready();
		}
		if (moving) idleSince = now;
		// Nothing is changing: stop drawing until something does.
		if (now - idleSince > 600 || !visible) {
			running = false;
			renderer.setAnimationLoop(null);
		}
	}

	function wake() {
		idleSince = performance.now();
		if (!running && visible && !disposed) {
			running = true;
			renderer.setAnimationLoop(frame);
		}
	}

	const watch = new IntersectionObserver((entries) => {
		visible = !!entries.at(-1)?.isIntersecting && document.visibilityState === 'visible';
		if (visible) wake();
	});
	watch.observe(stage);
	const onVisibility = () => {
		visible = document.visibilityState === 'visible';
		if (visible) wake();
	};
	document.addEventListener('visibilitychange', onVisibility);
	const sizes = new ResizeObserver(() => wake());
	sizes.observe(stage);

	// iOS drops a backgrounded page's context. Ask for it back, then light the gold again: the
	// environment map lived in the lost context.
	const lost = (e: Event) => e.preventDefault();
	const restored = () => {
		scene.environment = new PMREMGenerator(renderer).fromScene(new RoomEnvironment(), 0.04).texture;
		wake();
	};
	el.addEventListener('webglcontextlost', lost);
	el.addEventListener('webglcontextrestored', restored);

	// ------------------------------------------------------------------ start
	applyInside();
	pose(Infinity);
	resize();
	flyTo(wide, true);
	const api: ArkScene = {
		setInside(on) {
			inside = on;
			if (t0 === null) applyInside();
			wake();
		},
		setCloseUp(on) {
			closeUp = on;
			flyTo(on ? near : wide, false);
		},
		setChoice(id, option, choice) {
			if (id === 'rings' && option === 'position') {
				setRingHeight(ringHeight(choice));
				placeRingSpots();
			}
			wake();
		},
		select(id) {
			selected = id;
			highlight();
			wake();
		},
		build() {
			api.select(null);
			if (inside) applyInside();
			t0 = performance.now();
			lastStep = null;
			wake();
		},
		finish() {
			if (t0 === null) return;
			t0 = null;
			pose(Infinity);
			applyInside();
			lastStep = null;
			events.step(null);
			wake();
		},
		get building() {
			return t0 !== null;
		},
		hotspots: () => spots,
		onHotspots(fn) {
			listeners.push(fn);
		},
		dispose() {
			disposed = true;
			renderer.setAnimationLoop(null);
			watch.disconnect();
			sizes.disconnect();
			document.removeEventListener('visibilitychange', onVisibility);
			el.removeEventListener('pointerdown', onDown);
			el.removeEventListener('pointerup', onUp);
			el.removeEventListener('webglcontextlost', lost);
			el.removeEventListener('webglcontextrestored', restored);
			controls.dispose();
			scene.traverse((o) => {
				const m = o as Mesh;
				m.geometry?.dispose();
				const mat = m.material as Material | Material[] | undefined;
				[mat ?? []].flat().forEach((x) => x.dispose());
			});
			hammer.dispose();
			pmrem.dispose();
			renderer.dispose();
			el.remove();
		}
	};
	wake();
	return api;
}

/** A soft round glow for the light between the cherubim. */
function radial(): CanvasTexture {
	const c = document.createElement('canvas');
	c.width = c.height = 128;
	const x = c.getContext('2d')!;
	const g = x.createRadialGradient(64, 64, 0, 64, 64, 64);
	g.addColorStop(0, 'rgba(255,255,255,1)');
	g.addColorStop(0.35, 'rgba(255,240,200,0.35)');
	g.addColorStop(1, 'rgba(255,240,200,0)');
	x.fillStyle = g;
	x.fillRect(0, 0, 128, 128);
	return new CanvasTexture(c);
}

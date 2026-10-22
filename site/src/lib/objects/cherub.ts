/**
 * The cherubim's figures (the spec's `procedural:cherub`), from Marketing's prototype. The text gives
 * their material (beaten gold), their place (one at each end, of one piece with the mercy seat),
 * their wings (stretched on high, covering the seat) and their gaze (toward one another and the
 * seat): Exodus 25:18–20 and 37:7–9. The kneeling posture, faces, robes and feathers follow artistic
 * tradition, not the text, which is why the part is labelled an interpretive choice.
 *
 * The figure is drawn 1.12 cubits tall to the top of its wings and scaled to the spec's height.
 * Local frame: the origin on the mercy seat, +x toward the seat's centre, +y up, z sideways.
 */
import {
	BufferGeometry,
	CanvasTexture,
	CatmullRomCurve3,
	CylinderGeometry,
	ExtrudeGeometry,
	Float32BufferAttribute,
	Group,
	type Material,
	Matrix4,
	Mesh,
	RepeatWrapping,
	Shape,
	SphereGeometry,
	TubeGeometry,
	Vector3
} from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';

/** The figure's height as drawn, to the top of the wings. */
export const DRAWN_HEIGHT = 1.12;

const WING_EDGE: [number, number, number][] = [
	[-0.06, 0.55, 0.09],
	[-0.15, 0.8, 0.22],
	[0.08, 1.04, 0.31],
	[0.48, 1.12, 0.31],
	[0.8, 0.99, 0.2]
];

/** Which way a feather points along the wing (t from the shoulder to the tip). */
function featherDir(t: number): [number, number, number] {
	const a = [-0.3, -0.62, 0.7];
	const b = [0.95, -0.32, 0.25];
	const k = Math.pow(t, 1.4);
	const v = a.map((c, i) => c + (b[i] - c) * k);
	const m = Math.hypot(...v);
	return [v[0] / m, v[1] / m, v[2] / m];
}

/** Primaries and secondaries, coverts, and the small coverts along the edge. */
const ROWS = [
	{ n: 18, len: (t: number) => 0.26 + 0.3 * Math.pow(t, 1.1), w: 0.078 },
	{ n: 15, len: (t: number) => 0.15 + 0.17 * Math.pow(t, 1.1), w: 0.07 },
	{ n: 12, len: (t: number) => 0.07 + 0.1 * t, w: 0.056 }
];

function surface(fn: (u: number, v: number) => Vector3, nu: number, nv: number): BufferGeometry {
	const pos: number[] = [];
	const uv: number[] = [];
	const idx: number[] = [];
	for (let j = 0; j <= nv; j++)
		for (let i = 0; i <= nu; i++) {
			const p = fn(i / nu, j / nv);
			pos.push(p.x, p.y, p.z);
			uv.push((i / nu) * 3, (j / nv) * 2);
		}
	for (let j = 0; j < nv; j++)
		for (let i = 0; i < nu; i++) {
			const a = j * (nu + 1) + i;
			const b = a + 1;
			const c = a + nu + 1;
			const d = c + 1;
			idx.push(a, c, b, b, c, d);
		}
	const g = new BufferGeometry();
	g.setAttribute('position', new Float32BufferAttribute(pos, 3));
	g.setAttribute('uv', new Float32BufferAttribute(uv, 2));
	g.setIndex(idx);
	g.computeVertexNormals();
	return g;
}

interface Lathe {
	sx?: number;
	folds?: number;
	foldAmt?: number;
	lean?: number;
	pivotY?: number;
	ox?: number;
}

/** A body of revolution with drapery folds; `prof` is [radius, y] pairs, `sx` squashes its depth. */
function lathe(prof: [number, number][], { sx = 1, folds = 0, foldAmt = 0, lean = 0, pivotY = 0, ox = 0 }: Lathe = {}) {
	const at = (v: number) => {
		const f = v * (prof.length - 1);
		const i = Math.min(prof.length - 2, Math.floor(f));
		const t = f - i;
		return [prof[i][0] + (prof[i + 1][0] - prof[i][0]) * t, prof[i][1] + (prof[i + 1][1] - prof[i][1]) * t];
	};
	return surface(
		(u, v) => {
			const th = u * Math.PI * 2;
			const [r0, y] = at(v);
			const r = r0 * (1 + foldAmt * Math.sin(th * folds) * (1 - v * 0.6));
			let x = Math.cos(th) * r * sx + ox;
			let yy = y;
			if (lean) {
				const dy = yy - pivotY;
				x += Math.sin(lean) * dy;
				yy = pivotY + Math.cos(lean) * dy;
			}
			return new Vector3(x, yy, Math.sin(th) * r);
		},
		28,
		10
	);
}

function featherShape(w: number, l: number): Shape {
	const s = new Shape();
	s.moveTo(0, 0);
	s.quadraticCurveTo(w * 0.5, l * 0.2, w * 0.45, l * 0.62);
	s.quadraticCurveTo(w * 0.32, l * 0.95, 0, l);
	s.quadraticCurveTo(-w * 0.46, l * 0.72, -w * 0.5, l * 0.3);
	s.quadraticCurveTo(-w * 0.36, l * 0.04, 0, 0);
	return s;
}

const flat = (g: BufferGeometry) => (g.index ? g.toNonIndexed() : g);

/** A bump map of small dents, for gold beaten into shape (Exodus 25:18, "of beaten work"). */
export function hammeredTexture(): CanvasTexture {
	const c = document.createElement('canvas');
	c.width = c.height = 512;
	const x = c.getContext('2d')!;
	x.fillStyle = '#808080';
	x.fillRect(0, 0, 512, 512);
	let seed = 7;
	const rnd = () => (seed = (seed * 16807) % 2147483647) / 2147483647;
	for (let i = 0; i < 2600; i++) {
		const px = rnd() * 512;
		const py = rnd() * 512;
		const r = 4 + rnd() * 11;
		const g = x.createRadialGradient(px, py, 0, px, py, r);
		g.addColorStop(0, 'rgba(40,40,40,0.35)');
		g.addColorStop(0.7, 'rgba(128,128,128,0.08)');
		g.addColorStop(1, 'rgba(200,200,200,0.12)');
		x.fillStyle = g;
		x.beginPath();
		x.arc(px, py, r, 0, Math.PI * 2);
		x.fill();
	}
	const t = new CanvasTexture(c);
	t.wrapS = t.wrapT = RepeatWrapping;
	return t;
}

/** One kneeling cherub: the body in `mat`, the feathers (seen from both sides) in `featherMat`. */
export function buildCherub(mat: Material, featherMat: Material): Group {
	const g = new Group();
	const parts: BufferGeometry[] = [];
	// The base, one piece with the mercy seat (Exodus 25:19).
	parts.push(new CylinderGeometry(0.19, 0.22, 0.035, 32).translate(-0.02, 0.017, 0));
	// A kneeling robe with folds.
	parts.push(
		lathe(
			[
				[0.172, 0.03],
				[0.18, 0.07],
				[0.16, 0.15],
				[0.132, 0.24],
				[0.115, 0.32]
			],
			{ sx: 1.22, folds: 11, foldAmt: 0.045, ox: -0.03 }
		)
	);
	// The torso, leaning toward the seat.
	const lean = 0.28;
	parts.push(
		lathe(
			[
				[0.125, 0.22],
				[0.118, 0.3],
				[0.124, 0.38],
				[0.14, 0.47],
				[0.136, 0.55],
				[0.08, 0.61],
				[0.04, 0.645]
			],
			{ sx: 0.78, lean, pivotY: 0.32, ox: -0.02 }
		)
	);
	const top = new Vector3(-0.02 + Math.sin(lean) * 0.32, 0.32 + Math.cos(lean) * 0.32, 0);
	// The head, bowed toward the mercy seat ("toward the mercy seat shall the faces of the cherubims be").
	const head = top.clone().add(new Vector3(0.065, 0.07, 0));
	parts.push(new SphereGeometry(0.086, 28, 20).scale(0.95, 1.12, 0.88).rotateZ(-0.45).translate(head.x, head.y, head.z));
	parts.push(
		new SphereGeometry(0.095, 28, 16, Math.PI * 0.5, Math.PI, 0, Math.PI * 0.62)
			.scale(1, 1.1, 0.95)
			.rotateZ(-0.45)
			.translate(head.x - 0.012, head.y + 0.006, head.z)
	);
	// Arms folded forward, hands together.
	for (const zs of [-1, 1]) {
		const sh = top.clone().add(new Vector3(-0.03, -0.07, zs * 0.115));
		const curve = new CatmullRomCurve3([
			sh,
			new Vector3(sh.x + 0.07, sh.y - 0.12, zs * 0.14),
			new Vector3(sh.x + 0.17, sh.y - 0.07, zs * 0.06),
			new Vector3(sh.x + 0.2, sh.y - 0.02, zs * 0.018)
		]);
		parts.push(new TubeGeometry(curve, 24, 0.026, 12, false));
		const hand = curve.getPoint(1);
		parts.push(new SphereGeometry(0.027, 14, 10).scale(1.3, 1.4, 0.8).translate(hand.x, hand.y + 0.01, hand.z));
	}
	const body = new Mesh(mergeGeometries(parts.map(flat)), mat);
	body.castShadow = body.receiveShadow = true;
	g.add(body);

	// Wings stretched forward and on high, covering the mercy seat (Exodus 25:20).
	const feathers: BufferGeometry[] = [];
	for (const zs of [-1, 1]) {
		const edge = new CatmullRomCurve3(WING_EDGE.map(([x, y, z]) => new Vector3(x, y, z * zs)));
		// A membrane under the feathers, so no light shows between them.
		feathers.push(
			surface(
				(u, v) => {
					const p = edge.getPoint(0.04 + u * 0.96);
					const d = featherDir(u);
					const len = ROWS[0].len(u) * 0.7 * v;
					return new Vector3(p.x + d[0] * len, p.y + d[1] * len, p.z + d[2] * zs * len - zs * 0.004);
				},
				30,
				4
			)
		);
		ROWS.forEach((row, r) => {
			for (let i = 0; i < row.n; i++) {
				const t = 0.05 + 0.95 * (i / (row.n - 1));
				const p = edge.getPoint(t);
				const tangent = edge.getTangent(t);
				const d0 = featherDir(t);
				const d = new Vector3(d0[0], d0[1], d0[2] * zs).normalize();
				const xA = tangent.clone().sub(d.clone().multiplyScalar(tangent.dot(d))).normalize();
				const zA = new Vector3().crossVectors(xA, d).normalize();
				const geo = new ExtrudeGeometry(featherShape(row.w * (1 + 0.15 * t), row.len(t)), {
					depth: 0.005,
					bevelEnabled: false,
					curveSegments: 6
				});
				const m = new Matrix4().makeBasis(xA, d, zA);
				m.setPosition(p.clone().add(zA.clone().multiplyScalar(zs * (0.006 + r * 0.006))));
				geo.applyMatrix4(m);
				feathers.push(flat(geo));
			}
		});
	}
	const wings = new Mesh(mergeGeometries(feathers.map(flat)), featherMat);
	wings.castShadow = wings.receiveShadow = true;
	g.add(wings);
	return g;
}

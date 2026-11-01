import type { BezierContour, Shape } from "./types";
import { rotatePoint } from "./geometry";
import type { Point } from "./geometry";
export function absoluteContours(s: Shape): BezierContour[] {
  return (s.contours ?? []).map((c) => ({
    ...c,
    nodes: c.nodes.map((n) => {
      const p = (p: Point) =>
        rotatePoint(
          [s.x + p[0] * s.w, s.y + p[1] * s.h],
          [s.x + s.w / 2, s.y + s.h / 2],
          s.rotation,
        );
      return {
        point: p(n.point),
        incoming: p(n.incoming),
        outgoing: p(n.outgoing),
      };
    }),
  }));
}
export function normalizePath(s: Shape, contours: BezierContour[]): Shape {
  const pts = contours.flatMap((c) =>
    c.nodes.flatMap((n) => [n.point, n.incoming, n.outgoing]),
  );
  const x = pts.length ? Math.min(...pts.map((p) => p[0])) : 0,
    y = pts.length ? Math.min(...pts.map((p) => p[1])) : 0;
  const w = Math.max(0.001, ...pts.map((p) => p[0] - x)),
    h = Math.max(0.001, ...pts.map((p) => p[1] - y));
  const p = (p: Point): Point => [(p[0] - x) / w, (p[1] - y) / h];
  return {
    ...s,
    type: "path",
    x,
    y,
    w,
    h,
    rotation: 0,
    evenOdd: s.evenOdd ?? true,
    contours: contours.map((c) => ({
      ...c,
      nodes: c.nodes.map((n) => ({
        point: p(n.point),
        incoming: p(n.incoming),
        outgoing: p(n.outgoing),
      })),
    })),
  };
}
export function pathData(contours: BezierContour[]): string {
  return contours
    .map((c) => {
      if (!c.nodes.length) return "";
      let d = `M${c.nodes[0].point.join(" ")}`;
      for (let i = 1; i < c.nodes.length + Number(c.closed); i++) {
        const a = c.nodes[i - 1],
          b = c.nodes[i % c.nodes.length];
        d += `C${a.outgoing.join(" ")} ${b.incoming.join(" ")} ${b.point.join(" ")}`;
      }
      return d + (c.closed ? "Z" : "");
    })
    .join(" ");
}

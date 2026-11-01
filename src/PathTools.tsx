import { useEffect, useRef, useState } from "react";
import type { PointerEvent as Pointer } from "react";
import type {
  BezierContour,
  BezierNode,
  Doc,
  Layer,
  Operation,
  Shape,
  Tool,
} from "./types";
import type { Point } from "./geometry";
import { hitShape } from "./geometry";
import {inverse,transformPoint,matrixSvg} from "./groups";
import type {Matrix} from "./groups";
import { absoluteContours, normalizePath, pathData } from "./paths";
type Props = {
  doc: Doc;
  layer?: Layer;
  shape?: Shape;
  parentMatrix: Matrix;
  tool: Tool;
  zoom: number;
  color: string;
  secondary: string;
  filled: boolean;
  strokeWidth: number;
  opacity: number;
  busy: boolean;
  action: (op: Operation) => Promise<Doc | undefined>;
  select: (id: number | null) => void;
};
export default function PathTools(p: Props) {
  const svg = useRef<SVGSVGElement>(null),
    draft = useRef<BezierNode[]>([]),
    moving = useRef<{
      contour: number;
      node: number;
      part: keyof BezierNode;
      original: BezierContour[];
    } | null>(null);
  const [nodes, setNodes] = useState<BezierNode[]>([]),
    [edited, setEdited] = useState<BezierContour[] | null>(null),
    [chosen, setChosen] = useState<[number, number] | null>(null);
  const current = useRef(p);
  current.current = p;
  const contours =
    edited ?? (p.shape?.type === "path" ? absoluteContours(p.shape) : []);
  const changed = useRef<BezierContour[] | null>(null);
  const finish = async (closed: boolean) => {
    const q = current.current,
      points = draft.current;
    if (points.length < 2) return;
    draft.current = [];
    setNodes([]);
    moving.current = null;
    let target = q.layer;
    if (!target || target.kind !== "vector") {
      const d = await q.action({ type: "addLayer", kind: "vector" });
      target = d?.layers.find((l) => l.id === d.activeHint);
    }
    if (!target) return;
    const local = points.map((n) => {
      const pt = (p: Point): Point => [p[0] - target!.x, p[1] - target!.y];
      return {
        point: pt(n.point),
        incoming: pt(n.incoming),
        outgoing: pt(n.outgoing),
      };
    });
    const shape = normalizePath(
      {
        id: 0,
        type: "path",
        x: 0,
        y: 0,
        w: 1,
        h: 1,
        rotation: 0,
        fill: closed && q.filled ? q.color : null,
        stroke: closed ? q.secondary : q.color,
        strokeWidth: q.strokeWidth,
        opacity: q.opacity,
      },
      [{ nodes: local, closed }],
    );
    await q.action({ type: "shape", id: target.id, shape });
  };
  useEffect(() => {
    draft.current = [];
    setNodes([]);
    moving.current = null;
    setEdited(null);
    setChosen(null);
  }, [p.tool, p.doc.open, p.layer?.id, p.shape?.id]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (
        ["INPUT", "TEXTAREA", "SELECT"].includes(
          (e.target as HTMLElement).tagName,
        )
      )
        return;
      if (p.tool === "pen" && e.key === "Enter") {
        e.preventDefault();
        e.stopImmediatePropagation();
        void finish(e.shiftKey);
      }
      if (e.key === "Escape") {
        draft.current = [];
        setNodes([]);
        moving.current = null;
        changed.current = null;
        setEdited(null);
      }
      if (p.tool === "pen" && e.key === "Backspace") {
        e.preventDefault();
        e.stopImmediatePropagation();
        draft.current.pop();
        setNodes([...draft.current]);
      }
      if (p.tool === "nodes" && ["Delete", "Backspace"].includes(e.key)) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (chosen && p.shape && p.layer) {
          const cs = absoluteContours(p.shape);
          if (cs[chosen[0]].nodes.length <= 2) return;
          cs[chosen[0]].nodes.splice(chosen[1], 1);
          setChosen(null);
          void p.action({
            type: "shape",
            id: p.layer.id,
            shape: normalizePath(p.shape, cs),
          });
        }
      }
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  });
  const point = (e: Pointer): Point => {
    const r = svg.current!.getBoundingClientRect();
    return [(e.clientX - r.left) / p.zoom, (e.clientY - r.top) / p.zoom];
  };
  const down = (e: Pointer<SVGSVGElement>) => {
    if (p.busy || p.doc.animation) return;
    e.stopPropagation();
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    const pt = point(e);
    if (p.tool === "pen") {
      if (e.button === 2) {
        void finish(false);
        return;
      }
      if (e.button !== 0) return;
      if (
        draft.current.length >= 3 &&
        Math.hypot(
          pt[0] - draft.current[0].point[0],
          pt[1] - draft.current[0].point[1],
        ) <
          8 / p.zoom
      ) {
        void finish(true);
        return;
      }
      if (draft.current.length >= 20_000) return;
      draft.current.push({ point: pt, incoming: pt, outgoing: pt });
      setNodes([...draft.current]);
      moving.current = {
        contour: -1,
        node: draft.current.length - 1,
        part: "outgoing",
        original: [],
      };
      return;
    }
    const layerPoint: Point = [pt[0] - (p.layer?.x ?? 0), pt[1] - (p.layer?.y ?? 0)];
    const local=transformPoint(inverse(p.parentMatrix),layerPoint);
    for (let ci = 0; ci < contours.length; ci++)
      for (let ni = 0; ni < contours[ci].nodes.length; ni++) {
        const n = contours[ci].nodes[ni];
        const parts: (keyof BezierNode)[] =
          chosen?.[0] === ci && chosen[1] === ni
            ? ["incoming", "outgoing", "point"]
            : ["point"];
        // Anchors take priority when a collapsed handle lies on them.
        parts.sort((a) => (a === "point" ? -1 : 0));
        for (const part of parts)
          if (
            Math.hypot(local[0] - n[part][0], local[1] - n[part][1]) <=
            7 / p.zoom
          ) {
            setChosen([ci, ni]);
            moving.current = {
              contour: ci,
              node: ni,
              part,
              original: structuredClone(contours),
            };
            return;
          }
      }
    const shape = hitShape(p.layer?.shapes??[],layerPoint,5/p.zoom,true);
    p.select(shape?.id ?? null);
    setChosen(null);
  };
  const move = (e: Pointer) => {
    const g = moving.current;
    if (!g) return;
    const pt = point(e);
    if (g.contour === -1) {
      const n = draft.current[g.node];
      n.outgoing = pt;
      n.incoming = [2 * n.point[0] - pt[0], 2 * n.point[1] - pt[1]];
      setNodes([...draft.current]);
      return;
    }
    const cs = structuredClone(g.original),
      n = cs[g.contour].nodes[g.node],
      local: Point = transformPoint(inverse(p.parentMatrix),[pt[0] - (p.layer?.x ?? 0), pt[1] - (p.layer?.y ?? 0)]);
    if (g.part === "point") {
      const delta = [local[0] - n.point[0], local[1] - n.point[1]];
      for (const k of ["point", "incoming", "outgoing"] as const)
        n[k] = [n[k][0] + delta[0], n[k][1] + delta[1]];
    } else {
      n[g.part] = local;
      if (!e.altKey)
        n[g.part === "incoming" ? "outgoing" : "incoming"] = [
          2 * n.point[0] - local[0],
          2 * n.point[1] - local[1],
        ];
    }
    changed.current = cs;
    setEdited(cs);
  };
  const up = async () => {
    moving.current = null;
    if (changed.current && p.shape && p.layer) {
      const cs = changed.current;
      changed.current = null;
      await p.action({
        type: "shape",
        id: p.layer.id,
        shape: normalizePath(p.shape, cs),
      });
    }
    setEdited(null);
  };
  return (
    <svg
      ref={svg}
      className="path-editor"
      aria-label={p.tool === "pen" ? "Dessin de tracé" : "Nœuds du tracé"}
      width={p.doc.width}
      height={p.doc.height}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={() => void up()}
      onPointerCancel={() => {
        moving.current = null;
        changed.current = null;
        setEdited(null);
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <g
        transform={
          p.tool === "nodes"
            ? `translate(${p.layer?.x ?? 0} ${p.layer?.y ?? 0}) ${matrixSvg(p.parentMatrix)}`
            : undefined
        }
      >
        <path
          d={pathData(p.tool === "pen" ? [{ nodes, closed: false }] : contours)}
          fill="none"
          stroke="#368ddd"
          strokeWidth={1.5 / p.zoom}
        />
        {(p.tool === "pen" ? [{ nodes, closed: false }] : contours).map(
          (c, ci) =>
            c.nodes.map((n, ni) => (
              <g key={`${ci}:${ni}`}>
                {(p.tool === "pen" ||
                  (chosen?.[0] === ci && chosen[1] === ni)) &&
                  (["incoming", "outgoing"] as const).map((k) => (
                    <g key={k}>
                      <line
                        x1={n.point[0]}
                        y1={n.point[1]}
                        x2={n[k][0]}
                        y2={n[k][1]}
                        stroke="#368ddd"
                        strokeWidth={1 / p.zoom}
                      />
                      <circle
                        cx={n[k][0]}
                        cy={n[k][1]}
                        r={3 / p.zoom}
                        fill="white"
                        stroke="#368ddd"
                        strokeWidth={1 / p.zoom}
                      />
                    </g>
                  ))}
                <rect
                  x={n.point[0] - 3.5 / p.zoom}
                  y={n.point[1] - 3.5 / p.zoom}
                  width={7 / p.zoom}
                  height={7 / p.zoom}
                  fill={
                    chosen?.[0] === ci && chosen[1] === ni ? "#368ddd" : "white"
                  }
                  stroke="#368ddd"
                  strokeWidth={1 / p.zoom}
                />
              </g>
            )),
        )}
      </g>
    </svg>
  );
}

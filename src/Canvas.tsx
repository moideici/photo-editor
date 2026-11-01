import { useEffect, useRef, useState } from "react";
import {invoke} from '@tauri-apps/api/core';
import type { PointerEvent as Pointer } from "react";
import { ImagePlus } from "lucide-react";
import PathTools from "./PathTools";
import type { Doc, Layer, Operation, Shape, Tool } from "./types";
import { retouchTools } from "./types";
import { rotatePoint, hitShape, hitCandidates, selectedShapeHit, resizeShape } from "./geometry";
import {context,editMatrix,inverse,transformPoint,matrixSvg} from "./groups";
import type {Matrix} from "./groups";
import type { Point } from "./geometry";
export type SelectionMode = "replace" | "add" | "subtract" | "intersect";
interface Props {
  initialPan?:Point;
  onPan?:(p:Point)=>void;
  doc: Doc;
  image: string;
  selectionImage: string;
  tool: Tool;
  layer?: Layer;
  selected: number | null;
  size: number;
  opacity: number;
  color: string;
  secondary: string;
  strokeWidth: number;
  filled: boolean;
  tolerance: number;
  nozzle: string;
  nozzlePath: string | null;
  zoom: number;
  setZoom: (v: number) => void;
  fitToken: number;
  selectionMode: SelectionMode;
  busy: boolean;
  action: (op: Operation) => Promise<Doc | undefined>;
  selectShape: (id: number | null) => void;
  pick: (x: number, y: number) => void;
  addText: (x: number, y: number) => void;
  open: () => void;
}
interface Gesture {
  start: Point;
  end: Point;
  points: Point[];
  kind: Tool | "pan" | "resizeShape";
  pan: Point;
  shape?: Shape;
  mode: SelectionMode;
  matrix?: Matrix;
  pick?:Promise<void>;
  picked?:boolean;
  cancelled?:boolean;
}
const bounds = (a: Point, b: Point) => ({
  x: Math.min(a[0], b[0]),
  y: Math.min(a[1], b[1]),
  w: Math.abs(b[0] - a[0]),
  h: Math.abs(b[1] - a[1]),
});
const isSelection = (tool: string) =>
  [
    "select",
    "selectEllipse",
    "lasso",
    "polygon",
    "wand",
    "colorSelect",
  ].includes(tool);
export default function Canvas(p: Props) {
  const liveDoc=useRef(p.doc);liveDoc.current=p.doc;
  const pickEpoch=useRef(0);
  const [pickError,setPickError]=useState('');
  const viewport = useRef<HTMLDivElement>(null),
    board = useRef<HTMLDivElement>(null),
    preview = useRef<HTMLCanvasElement>(null);
  const drag = useRef<Gesture | null>(null),
    space = useRef(false),
    poly = useRef<Point[]>([]),
    polyMode = useRef<SelectionMode>("replace");
  const [gesture, setGesture] = useState<Gesture | null>(null),
    [pan, setPan] = useState<Point>(p.initialPan??[0, 0]),
    [cursor, setCursor] = useState<Point | null>(null);
  const [polygon, setPolygon] = useState<Point[]>([]),
    [viewSize, setViewSize] = useState<Point>([800, 600]);
  useEffect(()=>p.onPan?.(pan),[pan]);
  const selectedContext=context(p.layer?.shapes??[],p.selected),selected=selectedContext?.shape;
  const [source, setSource] = useState<Point | null>(null);
  const sourceTarget = useRef<Point | null>(null);
  useEffect(() => {
    setSource(null);
    sourceTarget.current = null;
  }, [p.layer?.id, p.doc.open]);
  useEffect(() => {
    if (gesture?.kind !== "airbrush") return;
    const timer = setInterval(() => {
      const g = drag.current;
      if (g?.kind === "airbrush") {
        g.points.push([...g.end]);
        setGesture({ ...g });
      }
    }, 50);
    return () => clearInterval(timer);
  }, [gesture?.kind]);
  const finishPolygon = useRef<() => void>(() => {});
  finishPolygon.current = () => {
    const points = poly.current.filter(
      (pt, i, all) =>
        i === 0 || pt[0] !== all[i - 1][0] || pt[1] !== all[i - 1][1],
    );
    if (points.length >= 3)
      void p.action({
        type: "selection",
        selection: { shape: { type: "polygon", points }, inverted: false },
        mode: polyMode.current,
      });
    poly.current = [];
    setPolygon([]);
  };
  const panFrame=useRef(JSON.stringify([p.doc.open,p.doc.width,p.doc.height,p.fitToken]));
  useEffect(() => {
    const key=JSON.stringify([p.doc.open,p.doc.width,p.doc.height,p.fitToken]);
    if(panFrame.current!==key){panFrame.current=key;setPan([0,0]);}
  }, [p.doc.open, p.doc.width, p.doc.height, p.fitToken]);
  useEffect(() => {
    poly.current = [];
    setPolygon([]);
    drag.current = null;
    pickEpoch.current++;
    setGesture(null);
  }, [p.tool, p.doc.open]);
  useEffect(() => {
    const node = viewport.current;
    if (!node) return;
    const observer = new ResizeObserver((entries) =>
      setViewSize([
        entries[0].contentRect.width,
        entries[0].contentRect.height,
      ]),
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (
        ["INPUT", "TEXTAREA", "SELECT"].includes(
          (e.target as HTMLElement).tagName,
        )
      )
        return;
      if (e.code === "Space") {
        space.current = true;
        e.preventDefault();
      }
      if(e.key==='Escape'){pickEpoch.current++;if(drag.current)drag.current.cancelled=true;drag.current=null;setGesture(null);}
      if (e.key === "Enter" && poly.current.length) {
        e.preventDefault();
        finishPolygon.current();
      }
      if (e.key === "Escape") {
        poly.current = [];
        setPolygon([]);
        drag.current = null;
        setGesture(null);
      }
      if (e.key === "Backspace" && poly.current.length) {
        e.preventDefault();
        poly.current = poly.current.slice(0, -1);
        setPolygon([...poly.current]);
      }
    };
    const up = () => {
      space.current = false;
    };
    const blur = () => {
      space.current = false;
      drag.current = null;
      pickEpoch.current++;
      setGesture(null);
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", blur);
    };
  }, []);
  useEffect(() => {
    const node = viewport.current;
    if (!node) return;
    const wheel = (e: WheelEvent) => {
      if (!p.doc.open || drag.current) return;
      e.preventDefault();
      const z = Math.max(
        0.05,
        Math.min(16, p.zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1)),
      );
      const r = node.getBoundingClientRect(),
        f = z / p.zoom,
        m: Point = [
          e.clientX - r.left - r.width / 2,
          e.clientY - r.top - r.height / 2,
        ];
      setPan((old) => [
        old[0] * f + m[0] * (1 - f),
        old[1] * f + m[1] * (1 - f),
      ]);
      p.setZoom(z);
    };
    node.addEventListener("wheel", wheel, { passive: false });
    return () => node.removeEventListener("wheel", wheel);
  }, [p.zoom, p.setZoom, p.doc.open]);
  useEffect(() => {
    const canvas = preview.current,
      ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    if (!gesture || !["brush", "eraser"].includes(gesture.kind)) return;
    ctx.strokeStyle = gesture.kind === "eraser" ? "#777777" : p.color;
    ctx.fillStyle = ctx.strokeStyle;
    ctx.globalAlpha = p.opacity * 0.75;
    ctx.lineWidth = p.size;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.beginPath();
    gesture.points.forEach((point, i) =>
      i ? ctx.lineTo(...point) : ctx.moveTo(...point),
    );
    ctx.stroke();
    if (gesture.points.length === 1) {
      ctx.beginPath();
      ctx.arc(...gesture.start, p.size / 2, 0, 2 * Math.PI);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
  }, [gesture, p.color, p.opacity, p.size]);
  const point = (e: Pointer): Point => {
    const r = board.current!.getBoundingClientRect();
    return [(e.clientX - r.left) / p.zoom, (e.clientY - r.top) / p.zoom];
  };
  const mode = (e: Pointer): SelectionMode =>
    e.ctrlKey && e.altKey
      ? "intersect"
      : e.ctrlKey
        ? "add"
        : e.altKey
          ? "subtract"
          : p.selectionMode;
  const down = (e: Pointer<HTMLDivElement>) => {
    if (!p.doc.open || p.busy || (e.target as HTMLElement).closest("button"))
      return;
    const epoch=++pickEpoch.current;setPickError('');
    if (e.button === 2) {
      if (poly.current.length) finishPolygon.current();
      return;
    }
    e.currentTarget.setPointerCapture(e.pointerId);
    const pt = point(e),
      m = mode(e);
    if (space.current || p.tool === "hand" || e.button === 1) {
      drag.current = {
        start: [e.clientX, e.clientY],
        end: pt,
        points: [],
        kind: "pan",
        pan,
        mode: m,
      };
      setGesture(drag.current);
      return;
    }
    if (p.tool === "picker") {
      p.pick(Math.floor(pt[0]), Math.floor(pt[1]));
      return;
    }
    if (p.tool === "zoom") {
      p.setZoom(Math.max(0.05, Math.min(16, p.zoom * (e.altKey ? 0.8 : 1.25))));
      return;
    }
    if (p.tool === "polygon") {
      if (
        poly.current.length >= 3 &&
        Math.hypot(pt[0] - poly.current[0][0], pt[1] - poly.current[0][1]) <
          8 / p.zoom
      ) {
        finishPolygon.current();
        return;
      }
      if (!poly.current.length) polyMode.current = m;
      poly.current.push(pt);
      setPolygon([...poly.current]);
      return;
    }
    if (p.tool === "wand" || p.tool === "colorSelect") {
      void p.action({
        type: "selectColor",
        x: Math.floor(pt[0]),
        y: Math.floor(pt[1]),
        tolerance: p.tolerance,
        contiguous: p.tool === "wand",
        mode: m,
      });
      return;
    }
    if (!p.layer && !isSelection(p.tool)) return;
    if (["clone", "heal"].includes(p.tool) && e.altKey) {
      setSource(pt);
      sourceTarget.current = null;
      return;
    }
    if (["clone", "heal"].includes(p.tool) && source && !sourceTarget.current)
      sourceTarget.current = pt;
    if (p.tool === "fill") {
      void p.action({
        type: "fill",
        id: p.layer!.id,
        x: Math.floor(pt[0]),
        y: Math.floor(pt[1]),
        color: p.color,
        tolerance: p.tolerance,
      });
      return;
    }
    if (p.tool === "text") {
      p.addText(...pt);
      return;
    }
    let shape: Shape | undefined,
      kind: Gesture["kind"] = p.tool;
    let candidates:Shape[]|undefined;
    if (p.tool === "move" && p.layer?.kind === "vector") {
      const local: Point = [pt[0] - p.layer.x, pt[1] - p.layer.y];
      const handle = selected
        ? transformPoint(editMatrix(selectedContext),rotatePoint(
            [selected.x + selected.w, selected.y + selected.h],
            [selected.x + selected.w / 2, selected.y + selected.h / 2],
            selected.rotation,
          ))
        : null;
      if (
        selected && !selected.textPath?.sourceId &&
        handle &&
        Math.hypot(local[0] - handle[0], local[1] - handle[1]) < 10 / p.zoom
      ) {
        shape = selected;
        kind = "resizeShape";
      } else {
        const onSelected=selectedContext&&selectedShapeHit(p.layer.shapes??[],selectedContext.shape.id,local,4/p.zoom);
        shape = !e.altKey&&selectedContext?.parentId!=null&&onSelected ? selected : hitShape(p.layer.shapes ?? [],local,4/p.zoom,e.altKey);
        const hasMask=(ss:Shape[]):boolean=>ss.some(s=>!!s.mask||!!s.filters||s.type==='text'||(s.children?hasMask(s.children):false));
        if(hasMask(p.layer.shapes??[])){
          candidates=hitCandidates(p.layer.shapes??[],local,4/p.zoom,e.altKey);
          if(!e.altKey&&selectedContext?.parentId!=null&&onSelected&&selected)candidates.unshift(selected);
          candidates=[...new Map(candidates.map(s=>[s.id,s])).values()];shape=undefined;
        }else p.selectShape(shape?.id ?? null);
      }
    }
    drag.current = {
      start: pt,
      end: pt,
      points: [pt],
      kind,
      pan,
      shape,
      matrix: shape?editMatrix(context(p.layer?.shapes??[],shape.id)):undefined,
      mode: m,
    };
    if(candidates&&p.layer){
      const g=drag.current,layer=p.layer,snapshot=p.doc;
      g.pick=invoke<number|null>('vector_mask_hit',{layerId:layer.id,version:layer.version,candidates:candidates.map(s=>s.id),point:[pt[0]-layer.x,pt[1]-layer.y]}).then(id=>{
        if(epoch!==pickEpoch.current||liveDoc.current!==snapshot){g.cancelled=true;return;}
        g.shape=id==null?undefined:context(layer.shapes??[],id)?.shape;
        g.matrix=g.shape?editMatrix(context(layer.shapes??[],g.shape.id)):undefined;
        g.picked=true;p.selectShape(id);if(drag.current===g)setGesture({...g});
      }).catch(error=>{g.cancelled=true;if(epoch===pickEpoch.current&&liveDoc.current===snapshot)setPickError(String(error));});
    }
    setGesture({ ...drag.current });
  };
  const move = (e: Pointer<HTMLDivElement>) => {
    if (!p.doc.open) return;
    const pt = point(e);
    setCursor(pt);
    const g = drag.current;
    if (!g) return;
    if (g.kind === "pan") {
      setPan([
        g.pan[0] + e.clientX - g.start[0],
        g.pan[1] + e.clientY - g.start[1],
      ]);
      return;
    }
    g.end = pt;
    if (["brush", "eraser", "lasso", ...retouchTools].includes(g.kind))
      g.points.push(pt);
    setGesture({ ...g });
  };
  const up = async () => {
    const g = drag.current;
    drag.current = null;
    if (!g) return;
    await g.pick;
    if(g.cancelled){if(!drag.current)setGesture(null);return;}
    if (g.kind === "pan") {
      setGesture(null);
      return;
    }
    const b = bounds(g.start, g.end),
      id = p.layer?.id;
    try {
      if (g.kind === "select" || g.kind === "selectEllipse")
        await p.action({
          type: "selection",
          selection:
            b.w >= 1 && b.h >= 1
              ? {
                  shape: {
                    type: g.kind === "select" ? "rect" : "ellipse",
                    ...b,
                  },
                  inverted: false,
                }
              : null,
          mode: g.mode,
        });
      else if (g.kind === "lasso") {
        if (g.points.length >= 3)
          await p.action({
            type: "selection",
            selection: {
              shape: { type: "polygon", points: g.points },
              inverted: false,
            },
            mode: g.mode,
          });
      } else if (!p.layer) return;
      else if (retouchTools.includes(g.kind))
        await p.action({
          type: "retouch",
          id,
          tool: g.kind,
          points: g.points,
          size: p.size,
          strength: p.opacity,
          sourceOffset:
            source && sourceTarget.current
              ? [
                  source[0] - sourceTarget.current[0],
                  source[1] - sourceTarget.current[1],
                ]
              : null,
          color: p.color,
          nozzle: p.nozzle === "custom" ? "star" : p.nozzle,
          nozzlePath: p.nozzlePath,
        });
      else if (g.kind === "brush" || g.kind === "eraser")
        await p.action({
          type: "stroke",
          id,
          points: g.points,
          size: p.size,
          color: p.color,
          opacity: p.opacity,
          erase: g.kind === "eraser",
        });
      else if (g.kind === "gradient")
        await p.action({
          type: "gradient",
          id,
          start: g.start,
          end: g.end,
          color: p.color,
          secondary: p.secondary,
        });
      else if (g.kind === "move" || g.kind === "resizeShape") {
        let delta: Point = [g.end[0] - g.start[0], g.end[1] - g.start[1]];
        if(g.matrix){const m=inverse(g.matrix);delta=transformPoint([m[0],m[1],m[2],m[3],0,0],delta);}
        if (Math.hypot(...delta) < 0.1) return;
        if(g.shape?.textPath?.sourceId!=null)return;
        if (g.shape)
          await p.action({
            type: "shape",
            id,
            shape:
              g.kind === "resizeShape"
                ? resizeShape(g.shape, delta)
                : {
                    ...g.shape,
                    x: g.shape.x + delta[0],
                    y: g.shape.y + delta[1],
                  },
          });
        else
          await p.action({
            type: "layer",
            id,
            x: p.layer.x + Math.round(delta[0]),
            y: p.layer.y + Math.round(delta[1]),
          });
      } else if (
        ["rect", "ellipse", "line"].includes(g.kind) &&
        b.w + b.h >= 2
      ) {
        let target = p.layer;
        if (target.kind !== "vector") {
          const next = await p.action({ type: "addLayer", kind: "vector" });
          const created = next?.layers.find((l) => l.id === next.activeHint);
          if (!created) return;
          target = created;
        }
        const line = g.kind === "line";
        await p.action({
          type: "shape",
          id: target.id,
          shape: {
            id: 0,
            type: g.kind,
            x: (line ? g.start[0] : b.x) - target.x,
            y: (line ? g.start[1] : b.y) - target.y,
            w: line ? g.end[0] - g.start[0] : b.w,
            h: line ? g.end[1] - g.start[1] : b.h,
            rotation: 0,
            fill: p.filled && !line ? p.color : null,
            stroke: line ? p.color : p.secondary,
            strokeWidth: p.strokeWidth,
            opacity: p.opacity,
            radius: 0,
          },
        });
      }
    } finally {
      if(!drag.current)setGesture(null);
    }
  };
  const b = gesture ? bounds(gesture.start, gesture.end) : null,
    selection = p.doc.selection?.shape;
  let previewDelta:Point=gesture?[gesture.end[0]-gesture.start[0],gesture.end[1]-gesture.start[1]]:[0,0];
  if(gesture?.matrix){const m=inverse(gesture.matrix);previewDelta=transformPoint([m[0],m[1],m[2],m[3],0,0],previewDelta);}
  const displayShape = gesture?.shape && gesture.shape.textPath?.sourceId==null
    ? gesture.kind === "resizeShape"
      ? resizeShape(gesture.shape, previewDelta)
      : {
          ...gesture.shape,
          x: gesture.shape.x + previewDelta[0],
          y: gesture.shape.y + previewDelta[1],
        }
    : selected;
  const ruler = (axis: 0 | 1) => {
    const length = viewSize[axis],
      extent = axis === 0 ? p.doc.width : p.doc.height,
      origin = length / 2 - (extent * p.zoom) / 2 + pan[axis],
      raw = 70 / p.zoom,
      base = 10 ** Math.floor(Math.log10(raw)),
      unit = raw / base,
      step = (unit <= 1 ? 1 : unit <= 2 ? 2 : unit <= 5 ? 5 : 10) * base;
    const first = Math.floor(-origin / (step * p.zoom)) * step,
      count = Math.ceil(length / (step * p.zoom)) + 2;
    return (
      <svg
        className={axis === 0 ? "ruler-top" : "ruler-left"}
        width={axis === 0 ? "100%" : 20}
        height={axis === 0 ? 20 : "100%"}
      >
        {Array.from({ length: count }, (_, i) => {
          const value = first + i * step,
            pos = origin + value * p.zoom;
          return (
            <g key={i}>
              {axis === 0 ? (
                <>
                  <line x1={pos} y1={14} x2={pos} y2={20} />
                  <text x={pos + 4} y={11}>
                    {Math.round(value * 100) / 100}
                  </text>
                </>
              ) : (
                <>
                  <line x1={14} y1={pos} x2={20} y2={pos} />
                  <text transform={`translate(10 ${pos + 4}) rotate(-90)`}>
                    {Math.round(value * 100) / 100}
                  </text>
                </>
              )}
            </g>
          );
        })}
      </svg>
    );
  };
  return (
    <div
      ref={viewport}
      className={`viewport tool-${p.tool}`}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={() => void up()}
      onPointerCancel={() => {
        pickEpoch.current++;
        drag.current = null;
        setGesture(null);
      }}
      onDoubleClick={() => {
        if (p.tool === "polygon") finishPolygon.current();
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {pickError&&<p className="canvas-pick-error" role="alert">{pickError}<button onClick={()=>setPickError('')}>Fermer</button></p>}
      {gesture?.pick&&!gesture.picked&&<span className="canvas-pick-status" role="status">Sélection…</span>}
      {!p.doc.open ? (
        <div className="welcome">
          <h1>Prisme</h1>
          <p>Ouvrez une image ou créez un document.</p>
          <button className="primary" onClick={p.open}>
            <ImagePlus size={16} /> Ouvrir une image <kbd>Ctrl O</kbd>
          </button>
        </div>
      ) : (
        <div
          className="board-anchor"
          style={{
            transform: `translate(calc(-50% + ${pan[0]}px),calc(-50% + ${pan[1]}px))`,
            width: p.doc.width * p.zoom,
            height: p.doc.height * p.zoom,
          }}
        >
          <div
            ref={board}
            className="board checker"
            style={{
              width: p.doc.width,
              height: p.doc.height,
              transform: `scale(${p.zoom})`,
            }}
          >
            {p.image && (
              <img
                className="composite"
                src={p.image}
                draggable={false}
                alt="Document en cours"
              />
            )}
            <canvas ref={preview} width={p.doc.width} height={p.doc.height} />
            {selection?.type === "mask" && p.selectionImage && (
              <img
                className="mask-overlay"
                src={p.selectionImage}
                draggable={false}
                alt="Contour de la sélection"
              />
            )}
            <svg
              className="overlay"
              width={p.doc.width}
              height={p.doc.height}
              viewBox={`0 0 ${p.doc.width} ${p.doc.height}`}
            >
              {selection && selection.type !== "mask" && (
                <g className="selection" strokeWidth={1 / p.zoom}>
                  {selection.type === "rect" ? (
                    <rect
                      x={selection.x}
                      y={selection.y}
                      width={selection.w}
                      height={selection.h}
                    />
                  ) : selection.type === "ellipse" ? (
                    <ellipse
                      cx={selection.x + selection.w / 2}
                      cy={selection.y + selection.h / 2}
                      rx={selection.w / 2}
                      ry={selection.h / 2}
                    />
                  ) : selection.type === "polygon" ? (
                    <polygon
                      points={selection.points
                        .map((pt) => pt.join(","))
                        .join(" ")}
                    />
                  ) : null}
                  {p.doc.selection?.inverted && (
                    <rect
                      x={0}
                      y={0}
                      width={p.doc.width}
                      height={p.doc.height}
                    />
                  )}
                </g>
              )}
              {displayShape && (
                <g
                  transform={`translate(${p.layer?.x ?? 0} ${p.layer?.y ?? 0}) ${matrixSvg(editMatrix(selectedContext))} rotate(${displayShape.rotation} ${displayShape.x + displayShape.w / 2} ${displayShape.y + displayShape.h / 2})`}
                  className="shape-outline"
                  strokeWidth={1 / p.zoom}
                >
                  <rect
                    x={Math.min(
                      displayShape.x,
                      displayShape.x + displayShape.w,
                    )}
                    y={Math.min(
                      displayShape.y,
                      displayShape.y + displayShape.h,
                    )}
                    width={Math.abs(displayShape.w)}
                    height={Math.abs(displayShape.h)}
                  />
                  {!displayShape.textPath?.sourceId&&<rect
                    className="resize-handle"
                    x={displayShape.x + displayShape.w - 4 / p.zoom}
                    y={displayShape.y + displayShape.h - 4 / p.zoom}
                    width={8 / p.zoom}
                    height={8 / p.zoom}
                  />}
                </g>
              )}
              {b &&
                gesture &&
                ["select", "selectEllipse", "rect", "ellipse"].includes(
                  gesture.kind,
                ) &&
                (gesture.kind === "selectEllipse" ||
                gesture.kind === "ellipse" ? (
                  <ellipse
                    cx={b.x + b.w / 2}
                    cy={b.y + b.h / 2}
                    rx={b.w / 2}
                    ry={b.h / 2}
                    fill={
                      gesture.kind === "ellipse" && p.filled ? p.color : "none"
                    }
                    fillOpacity={p.opacity}
                    stroke={isSelection(gesture.kind) ? "#777" : p.secondary}
                    strokeWidth={
                      isSelection(gesture.kind) ? 1 / p.zoom : p.strokeWidth
                    }
                  />
                ) : (
                  <rect
                    x={b.x}
                    y={b.y}
                    width={b.w}
                    height={b.h}
                    fill={
                      gesture.kind === "rect" && p.filled ? p.color : "none"
                    }
                    fillOpacity={p.opacity}
                    stroke={isSelection(gesture.kind) ? "#777" : p.secondary}
                    strokeWidth={
                      isSelection(gesture.kind) ? 1 / p.zoom : p.strokeWidth
                    }
                  />
                ))}
              {gesture && gesture.kind === "lasso" && (
                <polyline
                  points={gesture.points.map((pt) => pt.join(",")).join(" ")}
                  className="selection"
                  strokeWidth={1 / p.zoom}
                />
              )}
              {polygon.length > 0 && (
                <polyline
                  points={[...polygon, ...(cursor ? [cursor] : [])]
                    .map((pt) => pt.join(","))
                    .join(" ")}
                  className="selection"
                  strokeWidth={1 / p.zoom}
                />
              )}
              {gesture && ["line", "gradient"].includes(gesture.kind) && (
                <line
                  x1={gesture.start[0]}
                  y1={gesture.start[1]}
                  x2={gesture.end[0]}
                  y2={gesture.end[1]}
                  stroke={p.color}
                  strokeWidth={
                    gesture.kind === "line" ? p.strokeWidth : 1 / p.zoom
                  }
                />
              )}
              {cursor &&
                !gesture &&
                ["brush", "eraser", ...retouchTools].includes(p.tool) && (
                  <circle
                    className="brush-cursor"
                    cx={cursor[0]}
                    cy={cursor[1]}
                    r={p.size / 2}
                    strokeWidth={1 / p.zoom}
                  />
                )}
              {source && ["clone", "heal"].includes(p.tool) && (
                <g className="shape-outline" strokeWidth={1 / p.zoom}>
                  <circle cx={source[0]} cy={source[1]} r={5 / p.zoom} />
                  <path
                    d={`M${source[0] - 8 / p.zoom} ${source[1]}h${16 / p.zoom}M${source[0]} ${source[1] - 8 / p.zoom}v${16 / p.zoom}`}
                  />
                </g>
              )}
              {gesture && retouchTools.includes(gesture.kind) && (
                <polyline
                  points={gesture.points.map((pt) => pt.join(",")).join(" ")}
                  className="selection"
                  strokeWidth={1 / p.zoom}
                />
              )}
            </svg>
            {["pen", "nodes"].includes(p.tool) && (
              <PathTools
                doc={p.doc}
                layer={p.layer}
                shape={selected}
                parentMatrix={editMatrix(selectedContext)}
                tool={p.tool}
                zoom={p.zoom}
                color={p.color}
                secondary={p.secondary}
                filled={p.filled}
                strokeWidth={p.strokeWidth}
                opacity={p.opacity}
                busy={p.busy}
                action={p.action}
                select={p.selectShape}
              />
            )}
          </div>
        </div>
      )}
      {p.doc.open && (
        <>
          {ruler(0)}
          {ruler(1)}
          <div className="ruler-corner" />
          <div className="canvas-note">
            {p.doc.width} × {p.doc.height} px{" "}
            {cursor && ` · ${Math.floor(cursor[0])}, ${Math.floor(cursor[1])}`}
            {p.tool === "polygon" &&
              " · Entrée / double-clic : fermer · Retour arrière : retirer un sommet"}
          </div>
        </>
      )}
    </div>
  );
}

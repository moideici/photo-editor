import HelpCenter from "./HelpCenter";
import ClipPanel from "./ClipPanel";
import MaskPanel from "./MaskPanel";
import FilterPanel from "./FilterPanel";
import ImagePanel from "./ImagePanel";
import TextPathPanel from "./TextPathPanel";
import CommandSearch from "./CommandSearch";
import RasterPreview from "./RasterPreview";
import WorkspaceBar from "./WorkspaceBar";
import type {WorkspaceState} from "./WorkspaceBar";
import LayerList from "./LayerList";
import FontPanel from "./FontPanel";
import VectorizeDialog from "./VectorizeDialog";
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  ArrowDown,
  ArrowUp,
  Brush,
  Check,
  ChevronDown,
  Circle,
  Copy,
  Download,
  Eraser,
  Eye,
  EyeOff,
  FilePlus2,
  FolderOpen,
  Hand,
  History,
  Layers,
  Maximize,
  Merge,
  MousePointer2,
  PaintBucket,
  PenTool,
  Pipette,
  Plus,
  Redo2,
  Save,
  Scan,
  Slash,
  SlidersHorizontal,
  Square,
  Sun,
  Trash2,
  Type,
  Undo2,
  X,
  ZoomIn,
  ZoomOut,
  Blend,
  Lasso,
  Pentagon,
  Wand,
  Palette,
} from "lucide-react";
import Canvas from "./Canvas";
import ToolPalette from "./ToolPalette";
import CommandBars from "./CommandBars";
import PathPanel from "./PathPanel";
import GroupPanel from "./GroupPanel";
import PaintEditor from "./PaintEditor";
import {findShape,entries,siblings,context} from "./groups";
import CurvesEditor, { identityCurves } from "./CurvesEditor";
import AssetEditor, { assetDefaults } from "./AssetEditor";
import EffectsEditor, { emptyEffects } from "./EffectsEditor";
import PaletteEditor, { hexColor } from "./PaletteEditor";
import AnimationPlayer from "./AnimationPlayer";
import ActionsPanel from "./ActionsPanel";
import PluginsPanel from "./PluginsPanel";
import EightBfPanel from "./EightBfPanel";
import PflPanel from "./PflPanel";
import type { AssetOptions, AssetKind } from "./AssetEditor";
import type { CurvesData } from "./CurvesEditor";
import type { SelectionMode } from "./Canvas";
import { emptyDoc, retouchTools } from "./types";
import type { Doc, Operation, Shape, Tool, LayerEffects } from "./types";
import { filters } from "./filters";
import type { Filter } from "./filters";

function toolPreset(tool:string){try{const p=JSON.parse(localStorage.getItem('prisme-tool-'+tool)??'{}');const number=(k:string,d:number,min:number,max:number)=>Number.isFinite(p[k])?Math.max(min,Math.min(max,p[k])):d;return{size:number('size',24,1,1000),opacity:number('opacity',1,0,1),tolerance:number('tolerance',24,0,255),strokeWidth:number('strokeWidth',2,0,100)};}catch{return{size:24,opacity:1,tolerance:24,strokeWidth:2};}}
const tools: { id: Tool; name: string; key: string; icon: typeof Brush }[] = [
  { id: "move", name: "Déplacement", key: "V", icon: MousePointer2 },
  { id: "select", name: "Sélection rectangle", key: "M", icon: Scan },
  { id: "selectEllipse", name: "Sélection ellipse", key: "", icon: Circle },
  { id: "lasso", name: "Lasso", key: "", icon: Lasso },
  { id: "polygon", name: "Sélection polygone", key: "P", icon: Pentagon },
  { id: "wand", name: "Baguette magique", key: "W", icon: Wand },
  { id: "colorSelect", name: "Sélection par couleur", key: "", icon: Palette },
  { id: "brush", name: "Pinceau", key: "B", icon: Brush },
  { id: "eraser", name: "Gomme", key: "E", icon: Eraser },
  { id: "clone", name: "Tampon de clonage", key: "C", icon: Copy },
  { id: "heal", name: "Correcteur", key: "J", icon: Plus },
  { id: "smudge", name: "Doigt", key: "S", icon: Hand },
  { id: "dodge", name: "Éclaircir", key: "O", icon: Sun },
  { id: "burn", name: "Assombrir", key: "", icon: Circle },
  { id: "airbrush", name: "Aérographe", key: "A", icon: Brush },
  { id: "nozzle", name: "Tube", key: "N", icon: PenTool },
  { id: "fill", name: "Pot de peinture", key: "G", icon: PaintBucket },
  { id: "picker", name: "Pipette", key: "I", icon: Pipette },
  { id: "gradient", name: "Dégradé", key: "D", icon: Blend },
  { id: "text", name: "Texte", key: "T", icon: Type },
  { id: "rect", name: "Rectangle vectoriel", key: "U", icon: Square },
  { id: "ellipse", name: "Ellipse vectorielle", key: "", icon: Circle },
  { id: "line", name: "Ligne vectorielle", key: "L", icon: Slash },
  { id: "hand", name: "Main", key: "H", icon: Hand },
  { id: "zoom", name: "Zoom", key: "Z", icon: ZoomIn },
  { id: "blur", name: "Flou local", key: "", icon: Brush },
  { id: "pen", name: "Plume Bézier", key: "", icon: PenTool },
  { id: "nodes", name: "Édition des nœuds", key: "", icon: MousePointer2 },
];
const blends = [
  ["normal", "Normal"],
  ["multiply", "Produit"],
  ["screen", "Écran"],
  ["overlay", "Incrustation"],
  ["darken", "Obscurcir"],
  ["lighten", "Éclaircir"],
  ["difference", "Différence"],
  ["add", "Addition"],
];
const fileFilters = [
  {
    name: "Images et projets",
    extensions: [
      "prisme",
      "pfv",
      "png",
      "jpg",
      "jpeg",
      "webp",
      "bmp",
      "gif",
      "tif",
      "tiff",
      "ico",
      "svg",
      "psd",
      "avif",
      "heic",
      "heif",
      "raw",
      "cr2",
      "cr3",
      "nef",
      "nrw",
      "arw",
      "raf",
      "rw2",
      "orf",
      "pef",
      "srw",
      "dng",
      "3fr",
      "rwl",
    ],
  },
];
export type Dialog = {
  kind:
    | "new"
    | "resize"
    | "canvas"
    | "text"
    | "rename"
    | "export"
    | "filter"
    | "rotate"
    | "selectionModify"
    | "rememberSelection"
    | "asset"
    | "effects"
    | "indexed"
    | "transparency";

  title: string;
  values: Record<string, string | number | boolean>;
  filter?: Filter;
  curves?: CurvesData;
  asset?: AssetOptions;
  effects?: LayerEffects;
};
function IconButton({
  title,
  children,
  onClick,
  disabled = false,
  className = "",
}: {
  title: string;
  children: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  className?: string;
}) {
  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      onClick={onClick}
      disabled={disabled}
      className={`icon-button ${className}`}
    >
      {children}
    </button>
  );
}
function NumberField({
  label,
  value,
  onCommit,
  min,
  max,
  step = 1,
  disabled=false,
}: {
  label: string;
  value: number;
  onCommit: (n: number) => void;
  min?: number;
  max?: number;
  step?: number;
  disabled?:boolean;
}) {
  return (
    <label className="number-field">
      <span>{label}</span>
      <input
        type="number"
        disabled={disabled}
        key={value}
        defaultValue={Math.round(value * 100) / 100}
        min={min}
        max={max}
        step={step}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
        }}
        onBlur={(e) => {
          if(e.currentTarget.value===e.currentTarget.defaultValue)return;
          const n = e.currentTarget.valueAsNumber;
          if(Number.isFinite(n)){
            const next=Math.min(max ?? Infinity,Math.max(min ?? -Infinity,n));
            if(next!==value){onCommit(next);return;}
          }
          e.currentTarget.value=e.currentTarget.defaultValue;
        }}
      />
    </label>
  );
}
function OpacityControl({
  value,
  onCommit,
}: {
  value: number;
  onCommit: (n: number) => void;
}) {
  const [v, setV] = useState(value);
  useEffect(() => setV(value), [value]);
  return (
    <label className="range-field">
      <span>
        Opacité <b>{Math.round(v * 100)} %</b>
      </span>
      <input
        aria-label="Opacité du calque"
        type="range"
        min="0"
        max="100"
        value={Math.round(v * 100)}
        onChange={(e) => setV(+e.target.value / 100)}
        onPointerUp={() => {
          if (v !== value) onCommit(v);
        }}
        onKeyUp={() => {
          if (v !== value) onCommit(v);
        }}
      />
    </label>
  );
}
export default function App() {
  const [workspaceState,setWorkspaceState]=useState<WorkspaceState|null>(null);
  const viewPan=useRef<[number,number]>([0,0]);

  const [traceLayerId,setTraceLayerId] = useState<number|null>(null);
  const [doc, setDoc] = useState<Doc>(emptyDoc),
    [image, setImage] = useState(""),
    [selectionImage, setSelectionImage] = useState(""),
    [active, setActive] = useState<number | null>(null),
    [selected, setSelected] = useState<number | null>(null);
  const [tool, setToolState] = useState<Tool>("brush"),
    [color, setColorValue] = useState("#000000"),
    [secondary, setSecondary] = useState("#ffffff");
  const [size, setSize] = useState(toolPreset("brush").size),
    [opacity, setOpacity] = useState(toolPreset("brush").opacity),
    [strokeWidth, setStrokeWidth] = useState(toolPreset("brush").strokeWidth),
    [filled, setFilled] = useState(true),
    [tolerance, setTolerance] = useState(toolPreset("brush").tolerance);
  function setTool(t:Tool){const p=toolPreset(t);setToolState(t);setSize(p.size);setOpacity(p.opacity);setTolerance(p.tolerance);setStrokeWidth(p.strokeWidth);}
  useEffect(()=>localStorage.setItem('prisme-tool-'+tool,JSON.stringify({size,opacity,tolerance,strokeWidth})),[tool,size,opacity,tolerance,strokeWidth]);
  const [recentColors,setRecentColors]=useState<string[]>(()=>{try{return JSON.parse(localStorage.getItem('prisme-recent-colors')??'[]').filter((s:unknown)=>typeof s==='string'&&/^#[0-9a-f]{6}$/i.test(s)).slice(0,16);}catch{return[];}});
  useEffect(()=>{if(!/^#[0-9a-f]{6}$/i.test(color))return;const timer=setTimeout(()=>setRecentColors(old=>{const next=[color,...old.filter(c=>c!==color)].slice(0,16);localStorage.setItem('prisme-recent-colors',JSON.stringify(next));return next;}),350);return()=>clearTimeout(timer);},[color]);
  const [nozzle, setNozzle] = useState("star"),
    [nozzlePath, setNozzlePath] = useState<string | null>(null);
  const [paletteInk, setPaletteInk] = useState<number | null>(null),
    [playbackStopToken, setPlaybackStopToken] = useState(0);
  const displayFrame = useRef(0);
  function setColor(value: string) {
    setPaletteInk(null);
    setColorValue(value.slice(0, 7));
  }
  const [zoom, setZoom] = useState(1),
    [fitToken, setFitToken] = useState(0),
    [selectionMode, setSelectionMode] = useState<SelectionMode>("replace"),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [menu, setMenu] = useState<string | null>(null);
  const [dialog, setDialog] = useState<Dialog | null>(null),
    [theme, setTheme] = useState(
      () => localStorage.getItem("prisme-theme") ?? "light",
    );
  const [guard, setGuard] = useState<
    ((v: "save" | "discard" | "cancel") => void) | null
  >(null);
  const busyRef = useRef(false),
    current = useRef(doc);
  current.current = doc;
  const layer = doc.layers.find((l) => l.id === active),
    shape = findShape(layer?.shapes??[],selected);
  useEffect(() => {
    displayFrame.current = doc.animation?.current ?? 0;
  }, [doc.animation?.id, doc.animation?.current]);
  useEffect(() => {
    let cancelled = false;
    setSelectionImage("");
    if (isTauri() && doc.open && doc.selection?.shape.type === "mask")
      void invoke<string>("selection_overlay")
        .then((src) => {
          if (!cancelled) setSelectionImage(src);
        })
        .catch((e) => {
          if (!cancelled) setError(String(e));
        });
    return () => {
      cancelled = true;
    };
  }, [doc.selection, doc.width, doc.height, doc.open]);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("prisme-theme", theme);
  }, [theme]);
  useEffect(() => {
    document.documentElement.dataset.hideLayers=localStorage.getItem('prisme-hide-layers')??'false';
    document.documentElement.dataset.hideOptions=localStorage.getItem('prisme-hide-options')??'false';
    document.documentElement.style.setProperty('--layers-width',(localStorage.getItem('prisme-layers-width')??185)+'px');
    document.documentElement.style.setProperty('--options-width',(localStorage.getItem('prisme-options-width')??285)+'px');
    if (!isTauri()) return;
    let cancelled = false;
    void (async () => {
      const initial = await invoke<Doc>("document_info");
      if (cancelled) return;
      setDoc(initial);
      current.current = initial;
      setActive(initial.activeHint ?? initial.layers.at(-1)?.id ?? null);
      const tabs=await invoke<WorkspaceState>('workspace_control',{request:{kind:'status'}});if(!cancelled)setWorkspaceState(tabs);
      if (initial.open) {
        const rendered = await invoke<string>("render_document");
        if (!cancelled) {
          setImage(rendered);
          fit(initial);
        }
      }
    })().catch((e) => {
      if (!cancelled) setError(String(e));
    });
    return () => {
      cancelled = true;
    };
  }, []);
  useEffect(() => {
    if (isTauri())
      void getCurrentWindow().setTitle(
        `${doc.dirty ? "● " : ""}${doc.open ? doc.name + " — " : ""}Prisme`,
      );
  }, [doc.dirty, doc.name, doc.open]);
  useEffect(() => {
    if (error) {
      const t = setTimeout(() => setError(""), 10000);
      return () => clearTimeout(t);
    }
  }, [error]);

  async function action(operation: Operation): Promise<Doc | undefined> {
    if (busyRef.current) return;
    if (!isTauri()) {
      setError(
        "L’édition nécessite l’application Windows. Lancez « npm run tauri dev » dans le dossier Prisme.",
      );
      return;
    }
    busyRef.current = true;
    setPlaybackStopToken((v) => v + 1);
    setBusy(true);
    setError("");
    try {
      const animation = current.current.animation;
      if (
        animation &&
        displayFrame.current !== animation.current &&
        !["new", "open", "close", "animationFrame", "extractFrame"].includes(
          operation.type,
        )
      )
        await invoke("editor_command", {
          operation: { type: "animationFrame", index: displayFrame.current },
        });
      let next:Doc;
      if(['workspace','new','open','openSvgEditable','close'].includes(operation.type)){
        await invoke('workspace_control',{request:{kind:'view',view:{zoom,active,selected,pan:viewPan.current}}});
        const request=operation.type==='workspace'?operation.request:operation.type==='close'?{kind:'close',discard:true}:{kind:'open',operation};
        const result=await invoke<WorkspaceState&{document?:Doc}>('workspace_control',{request});
        setWorkspaceState(result);
        if(!result.document)return current.current;
        next=result.document;
        if(result.view){setZoom(result.view.zoom||1);viewPan.current=result.view.pan;setActive(result.view.active);setSelected(result.view.selected);}
      }else next=operation.type==='runAction'?await invoke<Doc>('run_action'):await invoke<Doc>('editor_command',{operation});
      setDoc(next);
      current.current = next;
      setActive(
        (old) =>
          (operation.type==='workspace'?null:next.activeHint) ??
          (next.layers.some((l) => l.id === old)
            ? old
            : (next.layers.at(-1)?.id ?? null)),
      );
      if (operation.type === "shape" && (operation.shape as Shape).id === 0) {
        setSelected(
          next.layers.find((l) => l.id === operation.id)?.shapes?.at(-1)?.id ??
            null,
        );
      } else if (operation.type === "openSvgEditable") {
        setSelected(next.layers[0]?.shapes?.[0]?.id ?? null);
        setTool("nodes");
      } else if (operation.type === "decomposeSvg") {
        const beforeShapes=doc.layers.find((l)=>l.id===operation.id)?.shapes??[];
        const afterShapes=next.layers.find((l)=>l.id===operation.id)?.shapes??[];
        const parentId=context(beforeShapes,operation.shapeId as number)?.parentId;
        const old = siblings(beforeShapes,operation.shapeId as number);
        const at = old.findIndex((s) => s.id === operation.shapeId);
        setSelected((parentId?findShape(afterShapes,parentId)?.children:afterShapes)?.[at]?.id??null);
      } else if (operation.type === "boolean") {
        const beforeShapes=doc.layers.find((l)=>l.id===operation.id)?.shapes??[];
        const afterShapes=next.layers.find((l)=>l.id===operation.id)?.shapes??[];
        const parentId=context(beforeShapes,operation.first as number)?.parentId;
        const old = siblings(beforeShapes,operation.first as number);
        const at = old.findIndex(
          (s) => s.id === operation.first || s.id === operation.second,
        );
        setSelected(
          (parentId?findShape(afterShapes,parentId)?.children:afterShapes)?.[at]?.id ??
            null,
        );
      } else
        setSelected((old) =>
          next.layers.some((l) => !!findShape(l.shapes??[],old))
            ? old
            : null,
        );
      if(operation.type==='open'||operation.type==='save'){
        await invoke('workspace_control',{request:{kind:'remember',path:operation.path}}).catch(e=>setError('Fichiers récents : '+String(e)));
      }
      if(operation.type==='save'||operation.type==='close')await invoke('workspace_control',{request:{kind:'checkpoint'}}).catch(e=>setError('Récupération : '+String(e)));
      void invoke<WorkspaceState>('workspace_control',{request:{kind:'status'}}).then(setWorkspaceState).catch(e=>setError(String(e)));
      if (!next.open) setImage("");
      else if (
        ![
          "selection",
          "invertSelection",
          "selectColor",
          "modifySelection",
          "rememberSelection",
          "restoreSelection",
          "forgetSelection",
          "importSelection",
          "save",
          "export",
        ].includes(operation.type)
      )
        setImage(await invoke<string>("render_document"));
      if (
        ["new", "open", "openSvgEditable", "paste", "pasteClipboard"].includes(
          operation.type,
        ) &&
        (!doc.open ||
          !!operation.asNew ||
          !["paste", "pasteClipboard"].includes(operation.type))
      )
        fit(next);
      return next;
    } catch (e) {
      setError(String(e));
      return undefined;
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }
  function fit(d = doc) {
    setFitToken((token) => token + 1);
    const width = Math.max(
        200,
        (document.querySelector(".viewport")?.clientWidth ??
          window.innerWidth - 415) - 40,
      ),
      height = Math.max(
        200,
        (document.querySelector(".viewport")?.clientHeight ??
          window.innerHeight - 120) - 40,
      );
    setZoom(Math.max(0.05, Math.min(1, width / d.width, height / d.height)));
  }
  async function saveProject(as = false) {
    const d = current.current;
    if (!d.open) return false;
    const path =
      (!as && d.path) ||
      (await save({
        title: "Enregistrer le projet Prisme",
        defaultPath: d.path ?? `${d.name.replace(/\.[^.]+$/, "")}.prisme`,
        filters: [{ name: "Projet Prisme", extensions: ["prisme"] }],
      }));
    if (!path) return false;
    return !!(await action({
      type: "save",
      path: path.toLowerCase().endsWith(".prisme") ? path : path + ".prisme",
    }));
  }
  async function mayReplace() {
    if (!current.current.dirty) return true;
    const answer = await new Promise<"save" | "discard" | "cancel">((resolve) =>
      setGuard(() => resolve),
    );
    setGuard(null);
    return answer === "discard" || (answer === "save" && (await saveProject()));
  }
  async function openFile(asLayer = false, editableSvg = false) {
    setMenu(null);
    if (!isTauri()) {
      setError(
        "Ouvrez Prisme avec « npm run tauri dev » pour accéder aux fichiers.",
      );
      return;
    }

    const path = await open({
      title: asLayer
        ? "Importer comme calque"
        : "Ouvrir une image ou un projet",
      multiple: false,
      filters: editableSvg
        ? [{ name: "SVG éditable", extensions: ["svg"] }]
        : fileFilters,
    });
    if (typeof path === "string")
      await action({
        type: editableSvg ? "openSvgEditable" : asLayer ? "import" : "open",
        path,
      });
  }
  async function paste(asNew = false) {
    if (asNew && !(await mayReplace())) return;
    await action({ type: "pasteClipboard", asNew });
  }
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const listeners: (() => void)[] = [];
    void getCurrentWindow()
      .onDragDropEvent((event) => {
        if (event.payload.type === "drop") {
          const paths = event.payload.paths;
          void (async () => {
            for (const path of paths) {
              if (!current.current.open) {
                await action({ type: "open", path });
              } else {
                await action({ type: "import", path });
              }
            }
          })();
        }
      })
      .then((fn) => (disposed ? fn() : listeners.push(fn)));
    void getCurrentWindow()
      .onCloseRequested((event) => {
        event.preventDefault();
        void (async () => {
          let state=await invoke<WorkspaceState>('workspace_control',{request:{kind:'status'}});
          while(state.tabs.some(t=>t.open)){
            const tab=state.tabs.find(t=>t.id===state.active&&t.open)??state.tabs.find(t=>t.open)!;
            if(tab.id!==state.active)await action({type:'workspace',request:{kind:'switch',id:tab.id}});
            if(!(await mayReplace()))return;
            if(!(await action({type:'close'})))return;
            state=await invoke<WorkspaceState>('workspace_control',{request:{kind:'status'}});
          }
          await getCurrentWindow().destroy();
        })();
      })
      .then((fn) => (disposed ? fn() : listeners.push(fn)));
    return () => {
      disposed = true;
      listeners.forEach((fn) => fn());
    };
  }, []);
  useEffect(() => {
    const handle = (e: KeyboardEvent) => {
      const input = ["INPUT", "TEXTAREA", "SELECT"].includes(
        (e.target as HTMLElement).tagName,
      );
      if (e.key === "Escape") {
        if(traceLayerId!==null){if(!busy)setTraceLayerId(null);return;}
        setMenu(null);
        if (dialog) setDialog(null);
        return;
      }
      if (input || dialog || guard || traceLayerId!==null) return;
      if (e.ctrlKey || e.metaKey) {
        const key = e.key.toLowerCase();
        if (
          ["n", "o", "s", "z", "y", "a", "d", "v", "c", "0", "1"].includes(key)
        )
          e.preventDefault();
        if (key === "n") newDialog();
        else if (key === "o") void openFile(e.shiftKey);
        else if (key === "s") void saveProject(e.shiftKey);
        else if (key === "z")
          void action({ type: e.shiftKey ? "redo" : "undo" });
        else if (key === "y") void action({ type: "redo" });
        else if (key === "a" && doc.open)
          void action({
            type: "selection",
            selection: {
              shape: { type: "rect", x: 0, y: 0, w: doc.width, h: doc.height },
              inverted: false,
            },
          });
        else if (key === "d")
          void action({ type: "selection", selection: null });
        else if (key === "v") void paste();
        else if (key === "c" && doc.open)
          void action({ type: "copyClipboard" });
        else if (key === "0") fit();
        else if (key === "1") setZoom(1);
      } else if (e.key === "Delete" && shape && layer && tool !== "nodes")
        void action({ type: "deleteShape", id: layer.id, shapeId: shape.id });
      else if (e.key === "[") setSize((n) => Math.max(1, n - 2));
      else if (e.key === "]") setSize((n) => Math.min(1000, n + 2));
      else {
        const t = tools.find((t) => t.key && t.key === e.key.toUpperCase());
        if (t) setTool(t.id);
        if (e.key.toLowerCase() === "x") {
          setColor(secondary);
          setSecondary(color);
        }
      }
    };
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  });
  useEffect(()=>{const timer=setInterval(()=>{if(!busyRef.current)void invoke<WorkspaceState>('workspace_control',{request:{kind:'checkpoint'}}).then(setWorkspaceState).catch(e=>setError('Récupération : '+String(e)));},60000);return()=>clearInterval(timer);},[]);
  function newDialog() {
    setMenu(null);
    setDialog({
      kind: "new",
      title: "Nouvelle image",
      values: { width: 1280, height: 800, background: "#ffffff" },
    });
  }
  function dimensionsDialog(kind: "resize" | "canvas") {
    setMenu(null);
    setDialog({
      kind,
      title: kind === "resize" ? "Redimensionner l’image" : "Zone de travail",
      values: { width: doc.width, height: doc.height, center: true },
    });
  }
  function filterDialog(f: Filter) {
    setMenu(null);
    if (!layer) return;
    if (
      f.params.length === 0 &&
      !f.editor &&
      !f.colors?.length &&
      !f.checks?.length
    ) {
      void action({ type: "filter", id: layer.id, name: f.id, params: {} });
      return;
    }
    setDialog({
      kind: "filter",
      title: f.label,
      filter: f,
      values: {
        ...Object.fromEntries(f.params.map((p) => [p.key, p.value])),
        ...Object.fromEntries((f.colors ?? []).map((p) => [p.key, p.value])),
        ...Object.fromEntries((f.checks ?? []).map((p) => [p.key, p.value])),
        ...(f.id === "levels" ? { channel: 0 } : {}),
      },
      curves: f.editor === "curves" ? identityCurves() : undefined,
    });
  }
  function textDialog(x: number, y: number) {
    setDialog({
      kind: "text",
      title: "Texte vectoriel",
      values: {
        x,
        y,
        text: "Votre texte",
        fontFamily: "Segoe UI",
        fontSize: 48,
        bold: false,
        italic: false,
      },
    });
  }
  const initialDialog=useRef<Dialog|null>(null);
  useEffect(()=>{if(dialog&&!initialDialog.current)initialDialog.current=structuredClone(dialog);if(!dialog)initialDialog.current=null;},[!!dialog]);
  async function submitDialog() {
    if (!dialog) return;
    const d = dialog,
      v = d.values;
    setDialog(null);
    if (d.kind === "new") {
      await action({
          type: "new",
          width: +v.width,
          height: +v.height,
          background: v.background || null,
        });
    } else if (d.kind === "indexed")
      await action({
        type: "convertIndexed",
        colors: +v.colors,
        dither: !!v.dither,
      });
    else if (d.kind === "transparency" && layer)
      await action({
        type: "transparency",
        id: layer.id,
        color: v.color,
        tolerance: +v.tolerance,
      });
    else if (d.kind === "effects" && d.effects && layer)
      await action({ type: "effects", id: layer.id, effects: d.effects });
    else if (d.kind === "asset" && d.asset && layer)
      await action({ type: "asset", id: layer.id, ...d.asset });
    else if (d.kind === "selectionModify")
      await action({
        type: "modifySelection",
        kind: v.kind,
        radius: +v.radius,
      });
    else if (d.kind === "rememberSelection")
      await action({ type: "rememberSelection", name: v.name });
    else if (d.kind === "rotate")
      await action({ type: "rotate", degrees: +v.degrees });
    else if (d.kind === "resize")
      await action({ type: "resize", width: +v.width, height: +v.height });
    else if (d.kind === "canvas")
      await action({
        type: "canvas",
        width: +v.width,
        height: +v.height,
        offsetX: v.center ? Math.round((+v.width - doc.width) / 2) : 0,
        offsetY: v.center ? Math.round((+v.height - doc.height) / 2) : 0,
      });
    else if (d.kind === "rename" && layer)
      await action({ type: "layer", id: layer.id, name: v.name });
    else if (d.kind === "filter" && layer)
      await action({
        type: "filter",
        id: layer.id,
        name: d.filter!.id,
        params: {
          ...v,
          ...Object.fromEntries(
            d.filter!.params.map((p) => [p.key, Number(v[p.key])]),
          ),
          ...(d.curves ?? {}),
        },
      });
    else if (d.kind === "export") {
      const ext = String(v.format),
        path = await save({
          title: "Exporter une image",
          defaultPath: `${doc.name.replace(/\.[^.]+$/, "")}.${ext}`,
          filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
        });
      if (path)
        await action({
          type: "export",
          path: path.toLowerCase().endsWith("." + ext)
            ? path
            : path + "." + ext,
          quality: +v.quality,
          optimized: !!v.optimized,
          precision: +(v.precision ?? 8),
        });
    } else if (d.kind === "text") {
      let target = layer;
      if (target?.kind !== "vector") {
        const next = await action({ type: "addLayer", kind: "vector" });
        target = next?.layers.find((l) => l.id === next.activeHint);
      }
      if (!target) return;
      await action({
        type: "shape",
        id: target.id,
        shape: {
          id: 0,
          type: "text",
          x: +v.x - target.x,
          y: +v.y - target.y,
          w: Math.max(1, String(v.text).length * +v.fontSize * 0.6),
          h: +v.fontSize * 1.3,
          text: v.text,
          fontFamily: v.fontFamily,
          fontSize: +v.fontSize,
          bold: v.bold,
          italic: v.italic,
          fill: color,
          stroke: null,
          strokeWidth: 0,
          opacity,
          rotation: 0,
        },
      });
    }
  }
  const updateValue = (key: string, value: string | number | boolean) =>
    setDialog((d) =>
      d ? { ...d, values: { ...d.values, [key]: value } } : null,
    );
  const editShape = (patch: Partial<Shape>) => {
    if('fontFamily' in patch&&!('fontOptions' in patch))patch.fontOptions=null;
    if ("fill" in patch && !("fillGradient" in patch)) patch.fillGradient=null;
    if ("fill" in patch && !("fillPattern" in patch)) patch.fillPattern=null;
    if ("stroke" in patch && !("strokeGradient" in patch)) patch.strokeGradient=null;
    if ("stroke" in patch && !("strokePattern" in patch)) patch.strokePattern=null;
    if (shape?.type === "text" && !shape.textPath && patch.fontSize && shape.fontSize) {
      const ratio = patch.fontSize / shape.fontSize;
      patch = { ...patch, w: shape.w * ratio, h: shape.h * ratio };
    }
    if (shape && layer)
      void action({
        type: "shape",
        id: layer.id,
        shape: { ...shape, ...patch },
      });
  };
  const item = (
    label: string,
    fn: () => void,
    shortcut?: string,
    disabled = false,
  ) => (
    <button
      key={label}
      disabled={disabled || busy}
      onClick={() => {
        setMenu(null);
        fn();
      }}
    >
      <span>{label}</span>
      {shortcut && <kbd>{shortcut}</kbd>}
    </button>
  );
  const layerAction = (type: string) => {
    if (layer) void action({ type, id: layer.id });
  };
  const openPfl = (kind: number | null = null) => {
    window.dispatchEvent(new CustomEvent("prisme:pfl-type", { detail: kind }));
    const panel = document.querySelector<HTMLDetailsElement>(".pfl-panel");
    if (panel) { panel.open = true; panel.scrollIntoView({ block: "nearest" }); }
  };
  return (
    <div
      className="app"
      onPointerDown={(e) => {
        if (!(e.target as HTMLElement).closest(".menus")) setMenu(null);
      }}
    >
      <header className="topbar">
        <div className="brand">
          <span>◭</span> prisme<span className="brand-dot">.</span>
        </div>
        <nav className="menus" aria-label="Menu principal">
          {[
            "Fichier",
            "Édition",
            "Image",
            "Calque",
            "Sélection",
            "Réglages",
            "Filtres",
            "Affichage",
            "Outils",
            "Fenêtre",
            "Aide",
          ].map((name) => (
            <div className="menu" key={name}>
              <button
                className={menu === name ? "open" : ""}
                onClick={() => setMenu(menu === name ? null : name)}
              >
                {name}
              </button>
              {menu === name && (
                <div className="dropdown">
                  {name === "Fichier" && (
                    <>
                      {item("Nouvelle image", newDialog, "Ctrl N")}
                      {item("Ouvrir…", () => void openFile(), "Ctrl O")}
                      {item(
                        "Ouvrir un SVG éditable…",
                        () => void openFile(false, true),
                      )}
                      {item(
                        "Importer comme calque…",
                        () => void openFile(true),
                        "Ctrl ⇧ O",
                        !doc.open,
                      )}
                      {item("Importer par plugin PFL…", () => openPfl(2))}
                      {item("Exporter par plugin PFL…", () => openPfl(3), "", !doc.open)}
                      <hr />
                      {item(
                        "Enregistrer le projet",
                        () => void saveProject(),
                        "Ctrl S",
                        !doc.open,
                      )}
                      {item(
                        "Enregistrer sous…",
                        () => void saveProject(true),
                        "Ctrl ⇧ S",
                        !doc.open,
                      )}
                      {item(
                        "Exporter…",
                        () =>
                          setDialog({
                            kind: "export",
                            title: "Exporter une image",
                            values: { format: "png", quality: 92 },
                          }),
                        "",
                        !doc.open,
                      )}
                      <hr />
                      {item(
                        "Fermer le document",
                        () =>
                          void (async () => {
                            if (await mayReplace())
                              await action({ type: "close" });
                          })(),
                        "",
                        !doc.open,
                      )}
                    </>
                  )}
                  {name === "Affichage" && (
                    <>
                      {item(
                        "Taille réelle",
                        () => setZoom(1),
                        "Ctrl 1",
                        !doc.open,
                      )}
                      {item(
                        "Adapter à la fenêtre",
                        () => fit(),
                        "Ctrl 0",
                        !doc.open,
                      )}
                      {item("Afficher / masquer la sélection", () =>
                        document
                          .querySelector(".app")
                          ?.classList.toggle("selection-hidden"),
                      )}
                      {item("Thème clair / sombre", () =>
                        setTheme(theme === "dark" ? "light" : "dark"),
                      )}
                    </>
                  )}
                  {name === "Outils" && (
                    <>
                      {item('Plugins Prisme',()=>{const p=document.querySelector<HTMLDetailsElement>('.plugins-panel');if(p){p.open=true;p.scrollIntoView({block:'nearest'});}})}
                      {item('Filtres Photoshop 8BF',()=>{const p=document.querySelector<HTMLDetailsElement>('.eightbf-panel');if(p){p.open=true;p.scrollIntoView({block:'nearest'});}})}
                      {item('Plugins PhotoFiltre PFL',()=>openPfl())}
                      {item('Outils PFL…',()=>openPfl(4))}
                      {item("Automatisation", () => {
                        const d =
                          document.querySelector<HTMLDetailsElement>(
                            ".actions-panel",
                          );
                        if (d) {
                          d.open = true;
                          d.scrollIntoView({ block: "nearest" });
                        }
                      })}
                      {item(
                        "Plume Bézier",
                        () => setTool("pen"),
                        "",
                        !doc.open,
                      )}
                      {item(
                        "Édition des nœuds",
                        () => setTool("nodes"),
                        "",
                        !doc.open,
                      )}
                    </>
                  )}
                  {name === "Fenêtre" && (
                    <>
                      {item(
                        doc.open ? doc.name : "Aucun document ouvert",
                        () =>
                          document
                            .querySelector<HTMLElement>(".viewport")
                            ?.focus(),
                        "",
                        !doc.open,
                      )}
                    </>
                  )}
                  {name === "Aide" && (
                    <div className="menu-help">
                      Prisme 1.5 · Ctrl O ouvrir · Ctrl S enregistrer. Plume :
                      clic pour un angle, glisser pour une courbe, cliquer le
                      premier point pour fermer, Entrée pour terminer. Nœuds :
                      glisser une ancre ou une poignée ; Alt rend les poignées
                      indépendantes.
                    </div>
                  )}
                  {name === "Édition" && (
                    <>
                      {item(
                        "Annuler",
                        () => void action({ type: "undo" }),
                        "Ctrl Z",
                        !doc.open || doc.historyIndex === 0,
                      )}
                      {item(
                        "Rétablir",
                        () => void action({ type: "redo" }),
                        "Ctrl ⇧ Z",
                        doc.historyIndex >= doc.history.length - 1,
                      )}
                      <hr />
                      {item(
                        "Copier l’image",
                        () => void action({ type: "copyClipboard" }),
                        "Ctrl C",
                        !doc.open,
                      )}
                      {item("Coller une image", () => void paste(), "Ctrl V")}
                      {item(
                        "Coller comme nouvelle image",
                        () => void paste(true),
                      )}
                    </>
                  )}
                  {name === "Image" && (
                    <>
                      {item("Plugins PFL pour l’image…", () => openPfl(1), "", !doc.open)}
                      {item(
                        "Redimensionner…",
                        () => dimensionsDialog("resize"),
                        "",
                        !doc.open,
                      )}
                      {item(
                        "Zone de travail…",
                        () => dimensionsDialog("canvas"),
                        "",
                        !doc.open,
                      )}
                      {item(
                        "Recadrer sur la sélection",
                        () => void action({ type: "crop" }),
                        "",
                        !doc.selection,
                      )}
                      <hr />
                      {[90, 180, 270].map((degrees) =>
                        item(
                          `Rotation ${degrees}°`,
                          () => void action({ type: "rotate", degrees }),
                          "",
                          !doc.open,
                        ),
                      )}
                      {item(
                        "Rotation libre…",
                        () =>
                          setDialog({
                            kind: "rotate",
                            title: "Rotation libre",
                            values: { degrees: 15 },
                          }),
                        "",
                        !doc.open,
                      )}
                      {item(
                        "Miroir horizontal",
                        () => void action({ type: "flip", horizontal: true }),
                        "",
                        !doc.open,
                      )}
                      {item(
                        "Miroir vertical",
                        () => void action({ type: "flip", horizontal: false }),
                        "",
                        !doc.open,
                      )}
                      <hr />
                      {item(
                        "Convertir en couleurs indexées…",
                        () =>
                          setDialog({
                            kind: "indexed",
                            title: "Couleurs indexées",
                            values: { colors: 256, dither: false },
                          }),
                        "",
                        !doc.open || !!doc.animation,
                      )}
                      {item(
                        "Passer en RVB",
                        () => void action({ type: "convertRgb" }),
                        "",
                        !doc.indexed,
                      )}
                      {item(
                        "Transparence par couleur…",
                        () =>
                          setDialog({
                            kind: "transparency",
                            title: "Transparence par couleur",
                            values: { color, tolerance: 0 },
                          }),
                        "",
                        layer?.kind !== "raster" || !!doc.animation,
                      )}
                    </>
                  )}
                  {name === "Calque" && (
                    <>
                      {item("Importer un tracé PhotoFiltre PFV…",()=>void(async()=>{const path=await open({title:"Importer un tracé PFV",multiple:false,filters:[{name:"Tracé PhotoFiltre",extensions:["pfv"]}]});if(typeof path==='string')await action({type:'importPfv',path});})(),"",!doc.open||!!doc.indexed||!!doc.animation)}
                      {item("Vectoriser le bitmap…",()=>layer&&setTraceLayerId(layer.id),"",layer?.kind!=="raster"||!!doc.indexed||!!doc.animation)}
                      {item(
                        "Fusionner jusqu’à ce calque",
                        () =>
                          layer &&
                          void action({ type: "mergePrefix", id: layer.id }),
                        "",
                        !layer || doc.layers[0]?.id === layer.id,
                      )}
                      {item(
                        "Effets du calque…",
                        () => {
                          if (layer)
                            setDialog({
                              kind: "effects",
                              title: "Effets du calque",
                              values: {},
                              effects: structuredClone(
                                layer.effects ?? emptyEffects(),
                              ),
                            });
                        },
                        "",
                        !layer || !!doc.indexed || !!doc.animation,
                      )}
                      {item(
                        "Retirer les effets",
                        () => {
                          if (layer)
                            void action({
                              type: "effects",
                              id: layer.id,
                              effects: emptyEffects(),
                            });
                        },
                        "",
                        !layer ||
                          (!layer.effects?.shadow &&
                            !layer.effects?.outline &&
                            !layer.effects?.bevel),
                      )}
                      <hr />
                      {(
                        ["mask", "frame", "pattern", "texture"] as AssetKind[]
                      ).map((kind, i) =>
                        item(
                          ["Masque…", "Cadre…", "Motif…", "Texture…"][i],
                          () =>
                            setDialog({
                              kind: "asset",
                              title: [
                                "Appliquer un masque",
                                "Appliquer un cadre",
                                "Remplir avec un motif",
                                "Appliquer une texture",
                              ][i],
                              values: {},
                              asset: assetDefaults(kind, color, secondary),
                            }),
                          "",
                          layer?.kind !== "raster",
                        ),
                      )}
                      <hr />
                      {item(
                        "Nouveau calque bitmap",
                        () => void action({ type: "addLayer", kind: "raster" }),
                        "",
                        !doc.open,
                      )}
                      {item(
                        "Nouveau calque vectoriel",
                        () => void action({ type: "addLayer", kind: "vector" }),
                        "",
                        !doc.open,
                      )}
                      <hr />
                      {item(
                        "Dupliquer",
                        () => layerAction("duplicateLayer"),
                        "",
                        !layer,
                      )}
                      {item(
                        "Renommer…",
                        () =>
                          setDialog({
                            kind: "rename",
                            title: "Renommer le calque",
                            values: { name: layer?.name ?? "" },
                          }),
                        "",
                        !layer,
                      )}
                      {item(
                        "Rastériser",
                        () => layerAction("rasterize"),
                        "",
                        layer?.kind !== "vector",
                      )}
                      {item(
                        "Fusionner vers le bas",
                        () => layerAction("mergeDown"),
                        "",
                        !layer || doc.layers[0]?.id === layer.id,
                      )}
                      {item(
                        "Aplatir l’image",
                        () => void action({ type: "flatten" }),
                        "",
                        !doc.open,
                      )}
                      {item(
                        "Supprimer",
                        () => layerAction("deleteLayer"),
                        "",
                        doc.layers.length < 2,
                      )}
                    </>
                  )}
                  {name === "Sélection" && (
                    <>
                      {["feather", "grow", "shrink", "border"].map((kind, i) =>
                        item(
                          [
                            "Contour progressif…",
                            "Dilater…",
                            "Contracter…",
                            "Sélectionner le contour…",
                          ][i],
                          () =>
                            setDialog({
                              kind: "selectionModify",
                              title: [
                                "Contour progressif",
                                "Dilater",
                                "Contracter",
                                "Contour",
                              ][i],
                              values: { kind, radius: 5 },
                            }),
                          "",
                          !doc.selection,
                        ),
                      )}
                      {item(
                        "Mémoriser la sélection…",
                        () =>
                          setDialog({
                            kind: "rememberSelection",
                            title: "Mémoriser la sélection",
                            values: {
                              name: `Sélection ${doc.savedSelections.length + 1}`,
                            },
                          }),
                        "",
                        !doc.selection,
                      )}
                      {doc.savedSelections.map((name) =>
                        item(
                          `Rappeler : ${name}`,
                          () => void action({ type: "restoreSelection", name }),
                        ),
                      )}
                      {doc.savedSelections.map((name) =>
                        item(
                          `Oublier : ${name}`,
                          () => void action({ type: "forgetSelection", name }),
                        ),
                      )}
                      <hr />
                      {item(
                        "Importer une sélection PhotoFiltre…",
                        () =>
                          void (async () => {
                            const path = await open({
                              title: "Importer une sélection PFS",
                              multiple: false,
                              filters: [
                                {
                                  name: "Sélection PhotoFiltre",
                                  extensions: ["pfs"],
                                },
                              ],
                            });
                            if (typeof path === "string")
                              await action({ type: "importSelection", path });
                          })(),
                        "",
                        !doc.open,
                      )}
                      <hr />
                      {item(
                        "Tout sélectionner",
                        () =>
                          void action({
                            type: "selection",
                            selection: {
                              shape: {
                                type: "rect",
                                x: 0,
                                y: 0,
                                w: doc.width,
                                h: doc.height,
                              },
                              inverted: false,
                            },
                          }),
                        "Ctrl A",
                        !doc.open,
                      )}
                      {item(
                        "Désélectionner",
                        () =>
                          void action({ type: "selection", selection: null }),
                        "Ctrl D",
                        !doc.selection,
                      )}
                      {item(
                        "Inverser",
                        () => void action({ type: "invertSelection" }),
                        "",
                        !doc.open,
                      )}
                    </>
                  )}
                  {(name === "Réglages" || name === "Filtres") &&
                    filters
                      .filter((f) => f.group === name)
                      .map((f) =>
                        item(
                          f.label,
                          () => filterDialog(f),
                          "",
                          layer?.kind !== "raster",
                        ),
                      )}
                  {name === "Filtres" && item("Plugins PFL de filtres…", () => openPfl(0), "", !doc.open)}
                </div>
              )}
            </div>
          ))}
        </nav>
        <div className="header-actions"><HelpCenter blocked={busy||!!dialog||!!guard||traceLayerId!==null} beforeInstall={async()=>{if(busyRef.current)return false;const state=await invoke<WorkspaceState>('workspace_control',{request:{kind:'status'}});for(const tab of state.tabs.filter(t=>t.open&&t.dirty)){if(!(await action({type:'workspace',request:{kind:'switch',id:tab.id}})))return false;if(!(await mayReplace())){await action({type:'workspace',request:{kind:'switch',id:state.active}});return false;}}return true;}}/>
          <IconButton
            title="Changer de thème"
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          >
            <Sun size={17} />
          </IconButton>
          <button
            className="export-button"
            disabled={!doc.open || busy}
            onClick={() =>
              setDialog({
                kind: "export",
                title: "Exporter une image",
                values: { format: "png", quality: 92 },
              })
            }
          >
            <Download size={15} /> Exporter
          </button>
        </div>
      </header>
      <CommandBars
        doc={doc}
        layer={layer}
        busy={busy}
        zoom={zoom}
        setZoom={setZoom}
        fit={() => fit()}
        action={action}
        newDocument={newDialog}
        openDocument={() => void openFile()}
        saveDocument={(as) => void saveProject(as)}
        paste={(as) => void paste(as)}
        dimensions={dimensionsDialog}
        dialog={setDialog}
        color={color}
        tool={setTool}
        filter={(id) => {
          const f = filters.find((f) => f.id === id);
          if (f) filterDialog(f);
        }}
      />
      <CommandSearch blocked={busy||!!dialog||!!guard||traceLayerId!==null} commands={[
        {id:'new',label:'Nouvelle image',shortcut:'Ctrl N',run:newDialog},{id:'open',label:'Ouvrir une image dans un onglet',shortcut:'Ctrl O',run:()=>void openFile()},
        {id:'save',label:'Enregistrer le projet',shortcut:'Ctrl S',disabled:!doc.open,run:()=>void saveProject()},
        {id:'undo',label:'Annuler',shortcut:'Ctrl Z',disabled:!doc.open||doc.historyIndex===0,run:()=>void action({type:'undo'})},
        {id:'redo',label:'Rétablir',shortcut:'Ctrl Maj Z',disabled:!doc.open||doc.historyIndex===doc.history.length-1,run:()=>void action({type:'redo'})},
        {id:'fit',label:'Adapter à la fenêtre',shortcut:'Ctrl 0',disabled:!doc.open,run:()=>fit()},
        {id:'100',label:'Zoom à 100 %',shortcut:'Ctrl 1',disabled:!doc.open,run:()=>setZoom(1)},
        {id:'vectorize',label:'Vectoriser le bitmap',disabled:layer?.kind!=='raster'||!!doc.indexed||!!doc.animation,run:()=>layer&&setTraceLayerId(layer.id)},
        {id:'toggle-layers',label:'Afficher ou replier le panneau des calques',run:()=>{const root=document.documentElement;root.dataset.hideLayers=root.dataset.hideLayers==='true'?'false':'true';localStorage.setItem('prisme-hide-layers',root.dataset.hideLayers);}},
        {id:'toggle-options',label:'Afficher ou replier le panneau des options',run:()=>{const root=document.documentElement;root.dataset.hideOptions=root.dataset.hideOptions==='true'?'false':'true';localStorage.setItem('prisme-hide-options',root.dataset.hideOptions);}},
        {id:'reset-tool',label:'Réinitialiser les réglages de l’outil',run:()=>{setSize(24);setOpacity(1);setTolerance(24);setStrokeWidth(2);}},
        ...tools.map(t=>({id:'tool-'+t.id,label:t.name,shortcut:t.key,run:()=>setTool(t.id)})),
        ...filters.map(f=>({id:'filter-'+f.id,label:'Réglage : '+f.label,disabled:layer?.kind!=='raster'||!!doc.animation,run:()=>filterDialog(f)}))
      ]}/>
      <main className="workspace">
        <aside className="layers-dock" aria-label="Calques">
          <section className="panel layers-panel">
            <h2>
              <span>
                <Layers size={14} /> Calques <small>{doc.layers.length}</small>
              </span>
              <div>
                <IconButton
                  title="Ajouter un calque bitmap"
                  disabled={!doc.open || busy}
                  onClick={() =>
                    void action({ type: "addLayer", kind: "raster" })
                  }
                >
                  <Plus size={16} />
                </IconButton>
                <IconButton
                  title="Ajouter un calque vectoriel"
                  disabled={!doc.open || busy}
                  onClick={() =>
                    void action({ type: "addLayer", kind: "vector" })
                  }
                >
                  <PenTool size={15} />
                </IconButton>
              </div>
            </h2>
            {layer && (
              <div className="layer-controls">
                <select
                  aria-label="Mode de fusion"
                  value={layer.blend}
                  onChange={(e) =>
                    void action({
                      type: "layer",
                      id: layer.id,
                      blend: e.target.value,
                    })
                  }
                >
                  {blends.map(([v, l]) => (
                    <option key={v} value={v}>
                      {l}
                    </option>
                  ))}
                </select>
                <OpacityControl
                  key={layer.id}
                  value={layer.opacity}
                  onCommit={(v) =>
                    void action({ type: "layer", id: layer.id, opacity: v })
                  }
                />
              </div>
            )}
            <LayerList key={workspaceState?.active??0} doc={doc} active={active} busy={busy} onSelect={id=>{setActive(id);setSelected(null);}} action={action} effects={()=>layer&&setDialog({kind:'effects',title:'Effets du calque',values:{},effects:structuredClone(layer.effects??emptyEffects())})}/>
            <div className="layer-footer">
              <IconButton
                title="Monter le calque"
                disabled={!layer || doc.layers.at(-1)?.id === layer.id || busy}
                onClick={() => {
                  if (layer)
                    void action({
                      type: "reorder",
                      id: layer.id,
                      index: doc.layers.indexOf(layer) + 1,
                    });
                }}
              >
                <ArrowUp size={16} />
              </IconButton>
              <IconButton
                title="Descendre le calque"
                disabled={!layer || doc.layers[0]?.id === layer.id || busy}
                onClick={() => {
                  if (layer)
                    void action({
                      type: "reorder",
                      id: layer.id,
                      index: doc.layers.indexOf(layer) - 1,
                    });
                }}
              >
                <ArrowDown size={16} />
              </IconButton>
              <IconButton
                title="Dupliquer le calque"
                disabled={!layer || busy}
                onClick={() => layerAction("duplicateLayer")}
              >
                <Copy size={15} />
              </IconButton>
              <IconButton
                title="Fusionner vers le bas"
                disabled={!layer || doc.layers[0]?.id === layer.id || busy}
                onClick={() => layerAction("mergeDown")}
              >
                <Merge size={15} />
              </IconButton>
              <div className="options-spacer" />
              <IconButton
                title="Supprimer le calque"
                disabled={doc.layers.length < 2 || busy}
                onClick={() => layerAction("deleteLayer")}
              >
                <Trash2 size={15} />
              </IconButton>
            </div>
          </section>
        </aside>
        <section className="editor-area">
          <WorkspaceBar value={workspaceState} busy={busy} action={action} close={()=>void(async()=>{if(await mayReplace())await action({type:'close'});})()}/>
          {doc.animation && (
            <AnimationPlayer
              animation={doc.animation}
              stopToken={playbackStopToken}
              busy={busy}
              onFrame={(index, src) => {
                displayFrame.current = index;
                setImage(src);
              }}
              onSelect={(index) =>
                void action({ type: "animationFrame", index })
              }
              onExtract={(index) =>
                void action({ type: "extractFrame", index })
              }
              onError={setError}
            />
          )}
          <Canvas key={workspaceState?.active??0} initialPan={workspaceState?.view?.pan??[0,0]} onPan={pan=>{viewPan.current=pan;}}
            doc={doc}
            image={image}
            selectionImage={selectionImage}
            nozzle={nozzle}
            nozzlePath={nozzle === "custom" ? nozzlePath : null}
            selectionMode={selectionMode}
            fitToken={fitToken}
            tool={tool}
            layer={layer}
            selected={selected}
            size={size}
            opacity={opacity}
            color={
              doc.indexed &&
              paletteInk !== null &&
              doc.indexed.palette[paletteInk]
                ? hexColor(doc.indexed.palette[paletteInk], true)
                : color
            }
            secondary={secondary}
            strokeWidth={strokeWidth}
            filled={filled}
            tolerance={tolerance}
            zoom={zoom}
            setZoom={setZoom}
            busy={busy}
            action={action}
            selectShape={setSelected}
            pick={(x, y) =>
              void invoke<string>("sample_color", { x, y })
                .then(setColor)
                .catch((e) => setError(String(e)))
            }
            addText={textDialog}
            open={() => void openFile()}
          />
          {!doc.open && (
            <div className="welcome-actions">
              <button onClick={newDialog}>
                <FilePlus2 size={16} /> Nouvelle image <kbd>Ctrl N</kbd>
              </button>
              <span>Un espace libre pour créer.</span>
            </div>
          )}
        </section>
        <aside className="sidebar" aria-label="Palette d’outils">
          <section className="panel colors-panel">
            <h2>
              Couleurs <Pipette size={14} />
            </h2>
            <div className="color-row">
              <input
                aria-label="Couleur active"
                type="color"
                value={color}
                onChange={(e) => setColor(e.target.value)}
              />
              <div>
                <span>Premier plan</span>
                <input
                  aria-label="Code couleur"
                  className="hex"
                  key={color}
                  defaultValue={color}
                  maxLength={7}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") e.currentTarget.blur();
                  }}
                  onBlur={(e) => {
                    if (/^#[0-9a-f]{6}$/i.test(e.target.value))
                      setColor(e.target.value);
                    else e.target.value = color;
                  }}
                />
              </div>
              <IconButton
                title="Permuter les couleurs (X)"
                onClick={() => {
                  setColor(secondary);
                  setSecondary(color);
                }}
              >
                <Redo2 size={15} />
              </IconButton>
            </div>
            <div className="secondary-color">
              <label>
                Arrière-plan{" "}
                <input
                  aria-label="Couleur secondaire"
                  type="color"
                  value={secondary}
                  onChange={(e) => setSecondary(e.target.value)}
                />
              </label>
              <button
                aria-label="Couleurs noir et blanc"
                onClick={() => {
                  setColor("#000000");
                  setSecondary("#ffffff");
                }}
              >
                N/B
              </button>
            </div>
            <div className="swatches">
              {[
                "#ffffff",
                "#15171d",
                "#ab91ff",
                "#7267ed",
                "#4fbfa8",
                "#f0bf75",
                "#e58296",
                "#5c9edc",
                "#e55a4f",
                "#92bb71",
                "#bca8dd",
                "#746f80",
              ].map((c) => (
                <button
                  key={c}
                  title={c}
                  aria-label={`Couleur ${c}`}
                  style={{ background: c }}
                  onClick={() => setColor(c)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    setSecondary(c);
                  }}
                />
              ))}
            </div>
          </section>
          <ToolPalette tool={tool} onTool={setTool} />
          <div className="tool-details">
            <div className="optionsbar">
              {tool === "pen" && (
                <span className="hint">
                  Clic : angle · glisser : courbe · premier point : fermer ·
                  Entrée : terminer · Échap : abandonner.
                </span>
              )}
              {tool === "nodes" && (
                <span className="hint">
                  Glissez une ancre ou sa poignée. Alt : poignée indépendante.
                  Suppr : retirer le nœud choisi.
                </span>
              )}
              <div className="tool-name">
                {(() => {
                  const t = tools.find((t) => t.id === tool)!;
                  return (
                    <>
                      <t.icon size={17} />
                      {t.name}
                    </>
                  );
                })()}
              </div>
              {["brush", "eraser", ...retouchTools].includes(tool) && (
                <>
                  <label>
                    Taille{" "}
                    <input
                      aria-label="Taille du pinceau"
                      type="number"
                      min={1}
                      max={1000}
                      value={size}
                      onChange={(e) =>
                        setSize(Math.max(1, Math.min(1000, +e.target.value)))
                      }
                    />{" "}
                    px
                  </label>
                  <label>
                    Opacité{" "}
                    <input
                      aria-label="Opacité de l’outil"
                      type="range"
                      min={1}
                      max={100}
                      value={Math.round(opacity * 100)}
                      onChange={(e) => setOpacity(+e.target.value / 100)}
                    />
                    <span>{Math.round(opacity * 100)} %</span>
                  </label>
                </>
              )}
              {["clone", "heal"].includes(tool) && (
                <span className="hint">
                  Alt + clic : source · clonage aligné
                </span>
              )}
              {tool === "nozzle" && (
                <>
                  <label>
                    Tube
                    <select
                      aria-label="Motif du tube"
                      value={nozzle}
                      onChange={(e) => setNozzle(e.target.value)}
                    >
                      <option value="star">Étoile</option>
                      <option value="leaf">Feuille</option>
                      <option value="bubble">Bulle</option>
                      {nozzlePath && (
                        <option value="custom">Image chargée</option>
                      )}
                    </select>
                  </label>
                  <button
                    onClick={() =>
                      void (async () => {
                        const path = await open({
                          title: "Charger une image pour le tube",
                          multiple: false,
                          filters: [
                            {
                              name: "Image",
                              extensions: ["png", "svg", "jpg", "webp"],
                            },
                          ],
                        });
                        if (typeof path === "string") {
                          setNozzlePath(path);
                          setNozzle("custom");
                        }
                      })()
                    }
                  >
                    Charger un tube…
                  </button>
                </>
              )}
              {["fill", "wand", "colorSelect"].includes(tool) && (
                <label>
                  Tolérance{" "}
                  <input
                    type="range"
                    min={0}
                    max={255}
                    value={tolerance}
                    onChange={(e) => setTolerance(+e.target.value)}
                  />
                  {tolerance}
                </label>
              )}
              {[
                "select",
                "selectEllipse",
                "lasso",
                "polygon",
                "wand",
                "colorSelect",
              ].includes(tool) && (
                <label>
                  Mode{" "}
                  <select
                    aria-label="Mode de sélection"
                    value={selectionMode}
                    onChange={(e) =>
                      setSelectionMode(e.target.value as SelectionMode)
                    }
                  >
                    <option value="replace">Remplacer</option>
                    <option value="add">Ajouter (Ctrl)</option>
                    <option value="subtract">Soustraire (Alt)</option>
                    <option value="intersect">Intersection (Ctrl Alt)</option>
                  </select>
                </label>
              )}
              {["rect", "ellipse", "line", "text", "pen"].includes(tool) && (
                <>
                  <label>
                    <input
                      type="checkbox"
                      checked={filled}
                      onChange={(e) => setFilled(e.target.checked)}
                    />{" "}
                    Remplissage
                  </label>
                  <label>
                    Contour{" "}
                    <input
                      type="number"
                      min={0}
                      max={100}
                      value={strokeWidth}
                      onChange={(e) =>
                        setStrokeWidth(Math.max(0, +e.target.value))
                      }
                    />{" "}
                    px
                  </label>
                </>
              )}
            </div>

            {doc.metadata &&
              (doc.metadata.hasExif || doc.metadata.notices.length > 0) && (
                <section className="panel import-info">
                  <h2>Informations du document</h2>
                  {doc.metadata.hasExif && (
                    <p className="hint">
                      EXIF conservé dans le projet et les exports PNG, JPEG,
                      WebP et AVIF.
                    </p>
                  )}
                  {doc.metadata.notices.map((message, i) => (
                    <p className="hint" key={i}>
                      {message}
                    </p>
                  ))}
                </section>
              )}
            <ActionsPanel
              doc={doc}
              busy={busy}
              onReplay={() => void action({ type: "runAction" })}
              onError={setError}
            />
            {!!doc.fontNotices?.length&&<section className="panel font-notices" aria-label="Polices du document"><h2>Polices du document</h2>{doc.fontNotices.map((s,i)=><p role="alert" key={i}>{s}</p>)}</section>}
            {doc.indexed && (
              <PaletteEditor
                value={doc.indexed}
                disabled={busy}
                onChange={(index, color) =>
                  void action({ type: "paletteColor", index, color })
                }
                onUse={(index) => {
                  setPaletteInk(index);
                  setColorValue(hexColor(doc.indexed!.palette[index]));
                }}
                onRgb={() => void action({ type: "convertRgb" })}
              />
            )}
            {layer?.kind === "vector" && (
              <section className="panel shape-panel">
                <h2>
                  <span>
                    <SlidersHorizontal size={14} /> Formes
                  </span>
                </h2>
                <select
                  aria-label="Forme active"
                  value={selected ?? ""}
                  onChange={(e) => {
                    setSelected(+e.target.value || null);
                    setTool("move");
                  }}
                >
                  <option value="">Sélectionner une forme</option>
                  {entries(layer.shapes??[]).map(({shape:s,depth}) => (
                    <option key={s.id} value={s.id}>
                      {"　".repeat(depth)}{s.type === "group" ? "Groupe" : s.type === "text" ? s.text : s.type} #{s.id}
                    </option>
                  ))}
                </select>
                <GroupPanel layer={layer} shape={shape} busy={busy} action={action} select={id=>{setSelected(id);setTool("move");}}/>
                {shape && (
                  <>
                    <div className="field-grid">
                      <NumberField
                        label="X"
                        disabled={shape.textPath?.sourceId!=null}
                        value={shape.x}
                        onCommit={(x) => editShape({ x })}
                      />
                      <NumberField
                        label="Y"
                        disabled={shape.textPath?.sourceId!=null}
                        value={shape.y}
                        onCommit={(y) => editShape({ y })}
                      />
                      <NumberField
                        label="Largeur"
                        disabled={shape.textPath?.sourceId!=null}
                        min={1}
                        value={shape.w}
                        onCommit={(w) => editShape({ w })}
                      />
                      <NumberField
                        label="Hauteur"
                        disabled={shape.textPath?.sourceId!=null}
                        min={1}
                        value={shape.h}
                        onCommit={(h) => editShape({ h })}
                      />
                      <NumberField
                        label="Rotation"
                        disabled={shape.textPath?.sourceId!=null}
                        value={shape.rotation}
                        onCommit={(rotation) => editShape({ rotation })}
                      />
                      {!["group","image"].includes(shape.type)&&<NumberField
                        label="Contour"
                        min={0}
                        value={shape.strokeWidth}
                        onCommit={(strokeWidth) => editShape({ strokeWidth })}
                      />}
                      <NumberField
                        label="Opacité forme (%)"
                        min={0}
                        max={100}
                        value={shape.opacity * 100}
                        onCommit={(v) => editShape({ opacity: v / 100 })}
                      />
                    </div>
                    {!["group","image"].includes(shape.type)&&<div className="shape-colors">
                      <label>
                        <input
                          type="checkbox"
                          checked={!!shape.fill || !!shape.fillGradient || !!shape.fillPattern}
                          onChange={(e) =>
                            editShape({ fill: e.target.checked ? color : null })
                          }
                        />{" "}
                        Fond{" "}
                        <input
                          type="color"
                          value={shape.fill ?? "#000000"}
                          onChange={(e) => editShape({ fill: e.target.value })}
                        />
                      </label>
                      <label>
                        <input
                          type="checkbox"
                          checked={!!shape.stroke || !!shape.strokeGradient || !!shape.strokePattern}
                          onChange={(e) =>
                            editShape({
                              stroke: e.target.checked ? secondary : null,
                            })
                          }
                        />{" "}
                        Trait{" "}
                        <input
                          type="color"
                          value={shape.stroke ?? "#ffffff"}
                          onChange={(e) =>
                            editShape({ stroke: e.target.value })
                          }
                        />
                      </label>
                    </div>}
                    {!["group","svg","image"].includes(shape.type)&&<PaintEditor key={`paint:${shape.id}`} shape={shape} disabled={busy} onChange={editShape}/>}
                    {shape.type === "group"&&<ClipPanel key={`clip:${shape.id}`} shape={shape} disabled={busy} onChange={editShape}/>}
                    {shape.type === "group"&&<MaskPanel key={`mask:${shape.id}`} shape={shape} disabled={busy} onChange={editShape}/>}
                    {shape.type === "group"&&<FilterPanel key={`filter:${shape.id}`} shape={shape} disabled={busy} onChange={editShape}/>}
                    {shape.type === "image"&&<ImagePanel key={`image:${shape.id}`} shape={shape} disabled={busy} onChange={editShape} action={o=>action({...o,id:layer.id})}/>}
                    {shape.type === "text" && (
                      <>
                        <textarea
                          aria-label="Texte de la forme"
                          key={shape.id + ":" + shape.text}
                          defaultValue={shape.text}
                          onBlur={(e) => {
                            if (e.target.value !== shape.text)
                              editShape({ text: e.target.value });
                          }}
                        />
                        <NumberField
                          label="Corps"
                          value={shape.fontSize ?? 48}
                          min={1}
                          max={4096}
                          onCommit={(fontSize) => editShape({ fontSize })}
                        />
                        <label>Police<input aria-label="Police du texte" value={shape.fontFamily??'Segoe UI'} onChange={e=>editShape({fontFamily:e.target.value})}/></label>
                        <label><input type="checkbox" aria-label="Texte gras" checked={shape.bold??false} onChange={e=>editShape({bold:e.target.checked})}/>Gras</label>
                        <label><input type="checkbox" aria-label="Texte italique" checked={shape.italic??false} onChange={e=>editShape({italic:e.target.checked})}/>Italique</label>
                        <TextPathPanel key={`text-path:${shape.id}`} shape={shape} layer={layer} disabled={busy} onChange={editShape}/>
                        <FontPanel key={`font:${shape.id}`} shape={shape} layer={layer} doc={doc} disabled={busy} onChange={editShape} action={action}/>
                      </>
                    )}
                    <PathPanel
                      key={shape.id}
                      shape={shape}
                      layer={layer}
                      action={action}
                      nodes={() => setTool("nodes")}
                    />
                    <button
                      className="subtle"
                      onClick={() =>
                        void action({
                          type: "deleteShape",
                          id: layer.id,
                          shapeId: shape.id,
                        })
                      }
                    >
                      <Trash2 size={13} /> Supprimer la forme
                    </button>
                  </>
                )}
                {!shape && (
                  <p className="hint">
                    Outil déplacement : cliquez sur une forme pour la déplacer
                    ou la redimensionner.
                  </p>
                )}
              </section>
            )}
            <PluginsPanel layer={layer} doc={doc} busy={busy} action={action} />
            <EightBfPanel layer={layer} doc={doc} busy={busy} action={action} color={color} secondary={secondary} />
            <PflPanel layer={layer} doc={doc} busy={busy} action={action} />
            <details className="comfort-panel"><summary>Couleurs récentes et atelier</summary><div className="recent-colors">{recentColors.map(c=><button key={c} title={c} aria-label={'Couleur récente '+c} style={{backgroundColor:c}} onClick={()=>setColor(c)}/>)}</div><button onClick={()=>{setColor(secondary);setSecondary(color);}}>Échanger premier et arrière-plan (X)</button><button onClick={()=>{setSize(24);setOpacity(1);setTolerance(24);setStrokeWidth(2);}}>Réinitialiser les réglages de l’outil</button><label>Largeur des calques<input aria-label="Largeur du panneau des calques" type="range" min="150" max="400" defaultValue={localStorage.getItem('prisme-layers-width')??'185'} onChange={e=>{document.documentElement.style.setProperty('--layers-width',e.target.value+'px');localStorage.setItem('prisme-layers-width',e.target.value);}}/></label><label>Largeur des options<input aria-label="Largeur du panneau des options" type="range" min="210" max="480" defaultValue={localStorage.getItem('prisme-options-width')??'285'} onChange={e=>{document.documentElement.style.setProperty('--options-width',e.target.value+'px');localStorage.setItem('prisme-options-width',e.target.value);}}/></label><button onClick={()=>{document.documentElement.style.removeProperty('--layers-width');document.documentElement.style.removeProperty('--options-width');localStorage.removeItem('prisme-layers-width');localStorage.removeItem('prisme-options-width');}}>Réinitialiser l’atelier</button></details>
            <section className="panel history-panel">
              <h2>
                <span>
                  <History size={14} /> Historique
                </span>
                <small>{doc.history.length} étapes</small>
              </h2>
              <div className="history-list">
                {doc.history.map((label, i) => (
                  <button
                    className={`${i === doc.historyIndex ? "current" : ""} ${i > doc.historyIndex ? "future" : ""}`}
                    key={i}
                    disabled={busy}
                    onClick={() => void action({ type: "jump", index: i })}
                  >
                    <span className="history-dot" />
                    {label}
                    {i === doc.historyIndex && <Check size={13} />}
                  </button>
                ))}
                {!doc.open && (
                  <div className="panel-empty">
                    Chaque geste garde sa trace.
                  </div>
                )}
              </div>
            </section>
          </div>
        </aside>
      </main>
      <footer className="statusbar">
        <span className={busy ? "working" : ""}>
          <i />
          {busy
            ? "Traitement en cours…"
            : doc.open
              ? `${doc.layers.length} calque(s) · ${layer?.kind === "vector" ? "Vectoriel" : "Bitmap"}`
              : "Prêt à créer"}
        </span>
        <span className="status-hint">
          {tool === "hand"
            ? "Glisser pour déplacer la vue"
            : "Espace : main · Molette : zoom"}
        </span>
        <div className="zoom-controls">
          <IconButton
            title="Zoom arrière"
            onClick={() => setZoom(Math.max(0.05, zoom / 1.25))}
          >
            <ZoomOut size={15} />
          </IconButton>
          <button onClick={() => setZoom(1)}>{Math.round(zoom * 100)} %</button>
          <IconButton
            title="Zoom avant"
            onClick={() => setZoom(Math.min(16, zoom * 1.25))}
          >
            <ZoomIn size={15} />
          </IconButton>
          <IconButton
            title="Adapter à la fenêtre (Ctrl 0)"
            onClick={() => fit()}
            disabled={!doc.open}
          >
            <Maximize size={14} />
          </IconButton>
        </div>
      </footer>
      {error && (
        <div className="toast" role="alert">
          <span>{error}</span>
          <button aria-label="Fermer le message" onClick={() => setError("")}>
            <X size={16} />
          </button>
        </div>
      )}
      {traceLayerId!==null&&<VectorizeDialog key={traceLayerId} doc={doc} layer={doc.layers.find(l=>l.id===traceLayerId)} busy={busy} onClose={()=>setTraceLayerId(null)} action={action}/>}
      {dialog && (
        <div
          className="modal-backdrop"
          onPointerDown={(e) => {
            if (e.target === e.currentTarget) setDialog(null);
          }}
        >
          <form
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-label={dialog.title}
            onSubmit={(e) => {
              e.preventDefault();
              void submitDialog();
            }}
          >
            <div className="modal-heading">
              <h2>{dialog.title}</h2>
              <IconButton title="Annuler" onClick={() => setDialog(null)}>
                <X size={18} />
              </IconButton>
            </div>
            {dialog.kind === "indexed" && (
              <>
                <label>
                  Nombre maximal de couleurs
                  <input
                    required
                    type="number"
                    min={2}
                    max={256}
                    value={+dialog.values.colors}
                    onChange={(e) => updateValue("colors", e.target.value)}
                  />
                </label>
                <label className="checkbox-field">
                  <input
                    type="checkbox"
                    checked={!!dialog.values.dither}
                    onChange={(e) => updateValue("dither", e.target.checked)}
                  />
                  Tramage de diffusion
                </label>
                <p className="hint">
                  Les calques visibles et leurs effets seront aplatis. Annuler
                  restaure les calques. La palette et sa transparence restent
                  modifiables ; PNG conserve les indices.
                </p>
              </>
            )}
            {['transparency','text'].includes(dialog.kind)&&<button type="button" disabled={!(window as unknown as {EyeDropper?:unknown}).EyeDropper} onClick={()=>{const Eye=(window as unknown as {EyeDropper:new()=>{open:()=>Promise<{sRGBHex:string}>}}).EyeDropper;void new Eye().open().then(r=>{if(dialog.kind==='transparency')setDialog(d=>d?{...d,values:{...d.values,color:r.sRGBHex}}:null);else setColor(r.sRGBHex);}).catch(()=>{});}}>Pipette du dialogue</button>}
            {dialog.kind === "transparency" && (
              <>
                <label>
                  Couleur à rendre transparente
                  <input
                    type="color"
                    value={String(dialog.values.color)}
                    onChange={(e) => updateValue("color", e.target.value)}
                  />
                </label>
                <label>
                  Tolérance
                  <input
                    required
                    type="number"
                    min={0}
                    max={255}
                    value={+dialog.values.tolerance}
                    onChange={(e) => updateValue("tolerance", e.target.value)}
                  />
                </label>
                <p className="hint">
                  {doc.indexed
                    ? "Modifie les entrées correspondantes de la palette, sur toute l’image."
                    : "Applique la transparence au calque bitmap actif, dans la sélection si elle existe."}
                </p>
              </>
            )}
            {dialog.kind === "asset" && dialog.asset && (
              <AssetEditor
                value={dialog.asset}
                onChange={(asset) =>
                  setDialog((d) => (d ? { ...d, asset } : null))
                }
              />
            )}
            {dialog.kind === "effects" && dialog.effects && layer && (
              <EffectsEditor
                value={dialog.effects}
                layerId={layer.id}
                onChange={(effects) =>
                  setDialog((d) => (d ? { ...d, effects } : null))
                }
              />
            )}
            {dialog.kind === "selectionModify" && (
              <label>
                Rayon (pixels)
                <input
                  required
                  autoFocus
                  type="number"
                  min={0}
                  max={256}
                  step={0.5}
                  value={+dialog.values.radius}
                  onChange={(e) => updateValue("radius", e.target.value)}
                />
              </label>
            )}
            {dialog.kind === "rememberSelection" && (
              <label>
                Nom
                <input
                  required
                  autoFocus
                  maxLength={128}
                  value={String(dialog.values.name)}
                  onChange={(e) => updateValue("name", e.target.value)}
                />
              </label>
            )}
            {["new", "resize", "canvas"].includes(dialog.kind) && (
              <>
                <div className="field-grid">
                  {["width", "height"].map((k) => (
                    <label key={k}>
                      {k === "width" ? "Largeur" : "Hauteur"}
                      <div className="unit-input">
                        <input
                          autoFocus={k === "width"}
                          required
                          type="number"
                          min={1}
                          max={16384}
                          value={Number(dialog.values[k])}
                          onChange={(e) => updateValue(k, e.target.value)}
                        />
                        <span>px</span>
                      </div>
                    </label>
                  ))}
                </div>
                <p className="hint">
                  Jusqu’à 16 mégapixels.{" "}
                  {dialog.kind === "resize" ? "Rééchantillonnage Lanczos." : ""}
                </p>
              </>
            )}
            {dialog.kind === "new" && (
              <label>
                Fond
                <select
                  value={String(dialog.values.background)}
                  onChange={(e) => updateValue("background", e.target.value)}
                >
                  <option value="#ffffff">Blanc</option>
                  <option value="">Transparent</option>
                  <option value="#15171d">Sombre</option>
                  <option value={color}>Couleur du premier plan</option>
                </select>
              </label>
            )}
            {dialog.kind === "canvas" && (
              <label className="check-label">
                <input
                  type="checkbox"
                  checked={!!dialog.values.center}
                  onChange={(e) => updateValue("center", e.target.checked)}
                />{" "}
                Centrer le contenu existant
              </label>
            )}
            {dialog.kind === "rotate" && (
              <label>
                Angle en degrés
                <input
                  autoFocus
                  required
                  type="number"
                  min={-360}
                  max={360}
                  step={0.1}
                  value={+dialog.values.degrees}
                  onChange={(e) => updateValue("degrees", e.target.value)}
                />
              </label>
            )}
            {dialog.kind === "rename" && (
              <label>
                Nom
                <input
                  autoFocus
                  required
                  value={String(dialog.values.name)}
                  onChange={(e) => updateValue("name", e.target.value)}
                />
              </label>
            )}
            {dialog.kind === "text" && (
              <>
                <label>
                  Votre texte
                  <textarea
                    autoFocus
                    required
                    value={String(dialog.values.text)}
                    onChange={(e) => updateValue("text", e.target.value)}
                  />
                </label>
                <div className="field-grid">
                  <label>
                    Police
                    <input
                      value={String(dialog.values.fontFamily)}
                      onChange={(e) =>
                        updateValue("fontFamily", e.target.value)
                      }
                    />
                  </label>
                  <label>
                    Corps
                    <input
                      required
                      type="number"
                      min={1}
                      max={4096}
                      value={+dialog.values.fontSize}
                      onChange={(e) => updateValue("fontSize", e.target.value)}
                    />
                  </label>
                </div>
                <div className="inline-checks">
                  {["bold", "italic"].map((k) => (
                    <label className="check-label" key={k}>
                      <input
                        type="checkbox"
                        checked={!!dialog.values[k]}
                        onChange={(e) => updateValue(k, e.target.checked)}
                      />
                      {k === "bold" ? "Gras" : "Italique"}
                    </label>
                  ))}
                </div>
              </>
            )}
            {dialog.kind === "export" && (
              <>
                <label>
                  Format
                  <select
                    value={String(dialog.values.format)}
                    onChange={(e) => updateValue("format", e.target.value)}
                  >
                    {[
                      "png",
                      "jpg",
                      "webp",
                      "bmp",
                      "tiff",
                      "avif",
                      "gif",
                      "svg",
                      "pdf",
                    ].map((f) => (
                      <option key={f} value={f}>
                        {f.toUpperCase()}
                      </option>
                    ))}
                  </select>
                </label>
                {["jpg", "avif"].includes(String(dialog.values.format)) && (
                  <label>
                    Qualité {String(dialog.values.format).toUpperCase()} :{" "}
                    {dialog.values.quality} %
                    <input
                      type="range"
                      min={1}
                      max={100}
                      value={+dialog.values.quality}
                      onChange={(e) => updateValue("quality", +e.target.value)}
                    />
                  </label>
                )}
                <p className="hint">
                  Le SVG conserve les calques vectoriels. Pour garder tous vos
                  calques éditables, enregistrez aussi un projet .prisme.
                  {dialog.values.format === "gif" &&
                    " GIF : toutes les images de l’animation, palette 256 couleurs par image, transparence binaire, durées arrondies à 10 ms."}
                  {dialog.values.format === "pdf" &&
                    " Le PDF conserve les formes et le texte compatibles ; les groupes avec effets sont préparés en bitmap. Les motifs complexes, dégradés répétés/miroirs, rayons de foyer ou fusions additives peuvent nécessiter une image du document entier. Échelle : 96 pixels par pouce ; SVG et .prisme gardent les objets éditables."}
                  {doc.metadata?.hasExif &&
                    !["png", "jpg", "webp", "avif"].includes(
                      String(dialog.values.format),
                    ) &&
                    " Les EXIF restent dans le projet ; ce format d’export ne les conserve pas."}
                </p>
                {dialog.values.format === "svg" && (
                  <>
                    <label className="check-label">
                      <input
                        type="checkbox"
                        checked={!!dialog.values.optimized}
                        onChange={(e) =>
                          updateValue("optimized", e.target.checked)
                        }
                      />
                      Optimiser le SVG
                    </label>
                    {!!dialog.values.optimized && (
                      <label>
                        Précision SVG
                        <input
                          type="number"
                          min={2}
                          max={8}
                          value={+(dialog.values.precision ?? 8)}
                          onChange={(e) =>
                            updateValue("precision", e.target.value)
                          }
                        />
                        <span className="hint">
                          Décimales des coordonnées ; une valeur réduite peut
                          modifier les détails.
                        </span>
                      </label>
                    )}
                  </>
                )}
              </>
            )}
            {dialog.kind==='filter'&&layer&&dialog.filter&&<RasterPreview revision={doc.revision} operation={{type:'filter',id:layer.id,name:dialog.filter.id,params:{...dialog.values,...Object.fromEntries(dialog.filter.params.map(p=>[p.key,Number(dialog.values[p.key])])),...(dialog.curves??{})}}} reset={()=>initialDialog.current&&setDialog(structuredClone(initialDialog.current))}/>}
            {dialog.kind === "filter" && dialog.curves && layer && (
              <CurvesEditor
                value={dialog.curves}
                layerId={layer.id}
                onChange={(curves) =>
                  setDialog((d) => (d ? { ...d, curves } : null))
                }
              />
            )}
            {dialog.kind === "filter" && dialog.filter?.id === "levels" && (
              <label>
                Canal
                <select
                  value={String(dialog.values.channel)}
                  onChange={(e) => updateValue("channel", +e.target.value)}
                >
                  <option value="0">RVB</option>
                  <option value="1">Rouge</option>
                  <option value="2">Vert</option>
                  <option value="3">Bleu</option>
                </select>
              </label>
            )}
            {dialog.kind === "filter" &&
              dialog.filter?.colors?.map((p) => (
                <label key={p.key}>
                  {p.label}
                  <input
                    type="color"
                    aria-label={p.label}
                    value={String(dialog.values[p.key])}
                    onChange={(e) => updateValue(p.key, e.target.value)}
                  />
                </label>
              ))}
            {dialog.kind === "filter" &&
              dialog.filter?.checks?.map((p) => (
                <label className="check-label" key={p.key}>
                  <input
                    type="checkbox"
                    checked={!!dialog.values[p.key]}
                    onChange={(e) => updateValue(p.key, e.target.checked)}
                  />
                  {p.label}
                </label>
              ))}
            {dialog.kind === "filter" &&
              dialog.filter!.params.map((p) => (
                <label className="filter-param" key={p.key}>
                  <span>
                    {p.label}
                    <input
                      required
                      type="number"
                      min={p.min}
                      max={p.max}
                      step={p.step}
                      value={+dialog.values[p.key]}
                      onChange={(e) => updateValue(p.key, e.target.value)}
                    />
                  </span>
                  <input
                    type="range"
                    min={p.min}
                    max={p.max}
                    step={p.step}
                    value={+dialog.values[p.key]}
                    onChange={(e) => updateValue(p.key, +e.target.value)}
                  />
                </label>
              ))}
            <div className="modal-actions">
              <button type="button" onClick={() => setDialog(null)}>
                Annuler
              </button>
              <button type="submit" className="primary">
                {dialog.kind === "export"
                  ? "Exporter…"
                  : dialog.kind === "new"
                    ? "Créer l’image"
                    : "Appliquer"}
              </button>
            </div>
          </form>
        </div>
      )}
      {guard && (
        <div className="modal-backdrop">
          <div className="modal">
            <h2>Enregistrer vos modifications ?</h2>
            <p>Le document contient des modifications non enregistrées.</p>
            <div className="modal-actions">
              <button onClick={() => guard("cancel")}>Annuler</button>
              <button onClick={() => guard("discard")}>Abandonner</button>
              <button className="primary" onClick={() => guard("save")}>
                Enregistrer
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

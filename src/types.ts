export interface BezierNode {
  point: [number, number];
  incoming: [number, number];
  outgoing: [number, number];
}
export interface BezierContour {
  nodes: BezierNode[];
  closed: boolean;
}
export interface Shape {
  fontOptions?:FontOptions|null;
  textPath?: TextPath | null;
  svgViewport?:boolean;
  svgUnclipped?:boolean;
  filters?:FilterChain|null;
  mask?:VectorMask|null;
  clip?:VectorClip|null;
  groupStyle?: {blend:string;isolate:boolean} | null;
  fillPattern?: SvgPattern | null;
  strokePattern?: SvgPattern | null;
  fillGradient?: Gradient | null;
  strokeGradient?: Gradient | null;
  id: number;
  type: "rect" | "ellipse" | "line" | "text" | "svg" | "path" | "group" | "image";
  asset?:ImageAsset;
  transform?: [number, number, number, number, number, number] | null;
  children?: Shape[];
  contours?: BezierContour[];
  evenOdd?: boolean;
  x: number;
  y: number;
  w: number;
  h: number;
  rotation: number;
  fill: string | null;
  stroke: string | null;
  strokeWidth: number;
  opacity: number;
  radius?: number;
  text?: string;
  fontFamily?: string;
  fontSize?: number;
  bold?: boolean;
  italic?: boolean;
  source?: string;
  intrinsicW?: number;
  intrinsicH?: number;
}
export interface TextPath {
  sourceId:number|null;contourIndex:number;contour:BezierContour;
  intrinsicW:number;intrinsicH:number;startOffset:number;percent:boolean;
  normalOffset:number;anchor:'start'|'middle'|'end';reverse:boolean;
}
export interface FontOptions{axes:Record<string,number>;opticalSizing:'auto'|'none';}
export interface FontAxis{tag:string;name:string;min:number;default:number;max:number;hidden:boolean;}
export interface FontInfo{requested:string;available:boolean;resolved:string|null;axes:FontAxis[];}
export interface TextStyle{name:string;template:Shape;}
export interface Gradient {
  geometry: {kind:"linear";start:[number,number];end:[number,number]} | {kind:"radial";center:[number,number];focus:[number,number];radius:number;focalRadius:number};
  units:"objectBoundingBox"|"userSpaceOnUse";
  transform:[number,number,number,number,number,number];
  spread:"pad"|"reflect"|"repeat";
  stops:{offset:number;color:string;opacity:number}[];
}
export interface ImageAsset {mime:string;data:string;width:number;height:number;rendering:string;}
export interface VectorClip {transform:Gradient["transform"];paths:Shape[];intersect?:VectorClip|null;}
export interface VectorMask {transform:Gradient["transform"];region:[number,number,number,number];mode:'alpha'|'luminance';children:Shape[];intersect?:VectorMask|null;}
export interface FilterResource {children:Shape[];}
export interface FilterPrimitive {kind:string;attributes:Record<string,string>;children:FilterPrimitive[];resource?:FilterResource|null;}
export interface SvgFilter {region:[number,number,number,number];primitives:FilterPrimitive[];}
export interface FilterChain {transform:Gradient['transform'];filters:SvgFilter[];}
export interface SvgPattern {
  units:Gradient["units"];
  transform:Gradient["transform"];
  x:number;y:number;width:number;height:number;
  contentWidth:number;contentHeight:number;
  children:Shape[];
}
export interface Layer {
  locks:{position:boolean;pixels:boolean;alpha:boolean};
  id: number;
  name: string;
  visible: boolean;
  opacity: number;
  blend: string;
  x: number;
  y: number;
  version: number;
  kind: "raster" | "vector";
  width: number;
  height: number;
  shapes: Shape[] | null;
  effects: LayerEffects;
}
export interface ShadowEffect {
  x: number;
  y: number;
  blur: number;
  spread: number;
  color: string;
  opacity: number;
}
export interface OutlineEffect {
  width: number;
  color: string;
  opacity: number;
  position: "outside" | "inside" | "center";
}
export interface BevelEffect {
  size: number;
  depth: number;
  angle: number;
  opacity: number;
  highlight: string;
  shade: string;
}
export interface LayerEffects {
  shadow: ShadowEffect | null;
  outline: OutlineEffect | null;
  bevel: BevelEffect | null;
}
export type SelectionShape =
  | { type: "rect" | "ellipse"; x: number; y: number; w: number; h: number }
  | { type: "polygon"; points: [number, number][] };
export type MaskShape = {
  type: "mask";
  width: number;
  height: number;
  data: string;
};
export interface Selection {
  shape: SelectionShape | MaskShape;
  inverted: boolean;
}
export interface Doc {
  revision:number;
  textStyles:TextStyle[];
  fontNotices:string[];
  open: boolean;
  width: number;
  height: number;
  name: string;
  path: string | null;
  dirty: boolean;
  layers: Layer[];
  selection: Selection | null;
  savedSelections: string[];
  indexed: IndexedInfo | null;
  animation: AnimationInfo | null;
  metadata: { hasExif: boolean; notices: string[] };
  history: string[];
  historyIndex: number;
  activeHint: number | null;
}
export interface IndexedInfo {
  palette: [number, number, number, number][];
  counts: number[];
}
export interface AnimationInfo {
  id: number;
  frameCount: number;
  delays: number[];
  current: number;
  loops: number;
}
export type Operation = { type: string; [key: string]: unknown };
export type Tool =
  | "move"
  | "brush"
  | "eraser"
  | "fill"
  | "picker"
  | "gradient"
  | "select"
  | "selectEllipse"
  | "lasso"
  | "polygon"
  | "wand"
  | "colorSelect"
  | "rect"
  | "ellipse"
  | "line"
  | "text"
  | "zoom"
  | "hand"
  | "clone"
  | "heal"
  | "smudge"
  | "dodge"
  | "burn"
  | "airbrush"
  | "nozzle"
  | "blur"
  | "pen"
  | "nodes";
export const retouchTools: readonly string[] = [
  "blur",
  "clone",
  "heal",
  "smudge",
  "dodge",
  "burn",
  "airbrush",
  "nozzle",
];
export const emptyDoc: Doc = {
  revision:0,
  textStyles:[],fontNotices:[],
  open: false,
  width: 0,
  height: 0,
  name: "",
  path: null,
  dirty: false,
  layers: [],
  selection: null,
  savedSelections: [],
  indexed: null,
  animation: null,
  metadata: { hasExif: false, notices: [] },
  history: [],
  historyIndex: 0,
  activeHint: null,
};

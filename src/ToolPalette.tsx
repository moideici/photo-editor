import {
  Brush,
  Layers,
  Hand,
  Pipette,
  Wand,
  Slash,
  PaintBucket,
  Eraser,
  Copy,
  Lasso,
  Pentagon,
  Scan,
  Circle,
  Plus,
  Square,
  Type,
  ZoomIn,
  Blend,
  MousePointer2,
  PenTool,
  Droplets,
  Spline,
  GitBranch,
  Palette,
} from "lucide-react";
import type { Tool } from "./types";
const cells: {
  tool?: Tool;
  label: string;
  icon: typeof Brush;
  key?: string;
}[] = [
  { tool: "select", label: "Sélection rectangle", icon: Scan, key: "M" },
  { tool: "move", label: "Déplacement", icon: Layers, key: "V" },
  { tool: "hand", label: "Main", icon: Hand, key: "H" },
  { tool: "picker", label: "Pipette", icon: Pipette, key: "I" },
  { tool: "wand", label: "Baguette magique", icon: Wand, key: "W" },
  { tool: "line", label: "Ligne vectorielle", icon: Slash, key: "L" },
  { tool: "fill", label: "Pot de peinture", icon: PaintBucket, key: "G" },
  { tool: "airbrush", label: "Aérographe", icon: Brush, key: "A" },
  { tool: "eraser", label: "Gomme", icon: Eraser, key: "E" },
  { tool: "brush", label: "Pinceau", icon: Brush, key: "B" },
  { label: "Pinceau avancé — indisponible", icon: Brush },
  { tool: "clone", label: "Tampon de clonage", icon: Copy, key: "C" },
  { tool: "blur", label: "Flou local", icon: Droplets },
  { tool: "smudge", label: "Doigt", icon: Hand, key: "S" },
  { tool: "heal", label: "Correcteur", icon: Plus, key: "J" },
  { label: "Déformation — indisponible", icon: MousePointer2 },
  { label: "Pinceau artistique — indisponible", icon: Brush },
  { tool: "nozzle", label: "Tube", icon: PenTool, key: "N" },
];
const extra: { tool: Tool; label: string; icon: typeof Brush; key?: string }[] =
  [
    { tool: "selectEllipse", label: "Sélection ellipse", icon: Circle },
    { tool: "lasso", label: "Lasso", icon: Lasso },
    { tool: "polygon", label: "Sélection polygone", icon: Pentagon, key: "P" },
    { tool: "colorSelect", label: "Sélection par couleur", icon: Palette },
    { tool: "rect", label: "Rectangle vectoriel", icon: Square, key: "U" },
    { tool: "ellipse", label: "Ellipse vectorielle", icon: Circle },
    { tool: "text", label: "Texte", icon: Type, key: "T" },
    { tool: "gradient", label: "Dégradé", icon: Blend, key: "D" },
    { tool: "dodge", label: "Éclaircir", icon: Plus, key: "O" },
    { tool: "burn", label: "Assombrir", icon: Circle },
    { tool: "pen", label: "Plume Bézier", icon: Spline },
    { tool: "nodes", label: "Édition des nœuds", icon: GitBranch },
    { tool: "zoom", label: "Zoom", icon: ZoomIn, key: "Z" },
  ];
export default function ToolPalette({
  tool,
  onTool,
}: {
  tool: Tool;
  onTool: (tool: Tool) => void;
}) {
  const button = (cell: (typeof cells)[number], index: number) => (
    <button
      type="button"
      key={index}
      aria-label={cell.label + (cell.key ? ` (${cell.key})` : "")}
      title={cell.label + (cell.key ? ` (${cell.key})` : "")}
      disabled={!cell.tool}
      className={`icon-button ${cell.tool === tool ? "active" : ""}`}
      aria-pressed={!!cell.tool && cell.tool === tool}
      onClick={() => cell.tool && onTool(cell.tool)}
    >
      <cell.icon size={18} />
    </button>
  );
  return (
    <section className="tool-palette" aria-label="Outils">
      <div className="tool-grid" aria-label="Outils principaux">
        {cells.map(button)}
      </div>
      <div className="tool-extras" aria-label="Sélections et outils vectoriels">
        {extra.map(button)}
      </div>
    </section>
  );
}

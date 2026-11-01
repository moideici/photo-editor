import {
  FilePlus2,
  FolderOpen,
  Save,
  SaveAll,
  Printer,
  ScanLine,
  Undo2,
  Redo2,
  Copy,
  Clipboard,
  ClipboardPlus,
  Palette,
  Image,
  Expand,
  Scan,
  Type,
  PenTool,
  Folder,
  Workflow,
  Settings,
  ZoomIn,
  ZoomOut,
  Maximize,
  Fullscreen,
  Sun,
  Contrast,
  Sigma as Gamma,
  Blend,
  ChartNoAxesColumn,
  SlidersHorizontal,
  Wand,
  Sparkles,
  Paintbrush,
  Droplets,
  Triangle,
  Focus,
  MoveHorizontal,
  MoveVertical,
  RotateCcw,
  RotateCw,
  Layers,
} from "lucide-react";
import type { Doc, Layer, Operation, Tool } from "./types";
import type { Dialog } from "./App";
import { assetDefaults } from "./AssetEditor";
type Props = {
  doc: Doc;
  layer?: Layer;
  busy: boolean;
  zoom: number;
  setZoom: (n: number) => void;
  fit: () => void;
  action: (op: Operation) => Promise<Doc | undefined>;
  newDocument: () => void;
  openDocument: () => void;
  saveDocument: (as: boolean) => void;
  paste: (asNew?: boolean) => void;
  dimensions: (kind: "resize" | "canvas") => void;
  dialog: (d: Dialog) => void;
  color: string;
  tool: (t: Tool) => void;
  filter: (id: string) => void;
};
export default function CommandBars(p: Props) {
  const disabled = !p.doc.open || p.busy;
  const button = (
    title: string,
    Icon: typeof Save,
    fn: () => void,
    off = false,
    sign?: string,
  ) => (
    <button
      type="button"
      title={title}
      aria-label={title}
      disabled={off || p.busy}
      onClick={fn}
      className="icon-button command-icon"
    >
      <Icon size={17} />
      {sign && <span>{sign}</span>}
    </button>
  );
  const sep = <span className="toolbar-separator" />;
  const command = (op: Operation) => () => void p.action(op);
  const adjustment = (id: string, params: Record<string, number>) => () => {
    if (p.layer)
      void p.action({ type: "filter", id: p.layer.id, name: id, params });
  };
  const raster = p.layer?.kind !== "raster" || disabled;
  return (
    <div className="command-bars">
      <div className="commandbar" aria-label="Commandes principales">
        {button("Nouvelle image (Ctrl N)", FilePlus2, p.newDocument)}
        {button("Ouvrir une image (Ctrl O)", FolderOpen, p.openDocument)}
        {button(
          "Enregistrer le projet (Ctrl S)",
          Save,
          () => p.saveDocument(false),
          disabled,
        )}
        {button(
          "Enregistrer sous (Ctrl Maj S)",
          SaveAll,
          () => p.saveDocument(true),
          disabled,
        )}
        {sep}
        {button("Impression — indisponible", Printer, () => {}, true)}
        {button("Acquisition scanner — indisponible", ScanLine, () => {}, true)}
        {sep}
        {button(
          "Annuler (Ctrl Z)",
          Undo2,
          command({ type: "undo" }),
          disabled || p.doc.historyIndex === 0,
        )}
        {button(
          "Rétablir (Ctrl Maj Z)",
          Redo2,
          command({ type: "redo" }),
          disabled || p.doc.historyIndex >= p.doc.history.length - 1,
        )}
        {sep}
        {button(
          "Copier l’image (Ctrl C)",
          Copy,
          command({ type: "copyClipboard" }),
          disabled,
        )}
        {button("Coller une image (Ctrl V)", Clipboard, () => p.paste())}
        {button("Coller en tant que nouvelle image", ClipboardPlus, () =>
          p.paste(true),
        )}
        {sep}
        {button(
          "Mode RVB",
          Image,
          command({ type: "convertRgb" }),
          !p.doc.indexed,
        )}
        {button(
          "Couleurs indexées",
          Palette,
          () =>
            p.dialog({
              kind: "indexed",
              title: "Couleurs indexées",
              values: { colors: 256, dither: false },
            }),
          disabled || !!p.doc.animation,
        )}
        {button(
          "Transparence par couleur",
          Blend,
          () =>
            p.dialog({
              kind: "transparency",
              title: "Transparence par couleur",
              values: { color: p.color, tolerance: 0 },
            }),
          raster || !!p.doc.animation,
        )}
        {sep}
        {button(
          "Taille de l’image",
          Image,
          () => p.dimensions("resize"),
          disabled,
        )}
        {button(
          "Zone de travail",
          Expand,
          () => p.dimensions("canvas"),
          disabled,
        )}
        {button(
          "Afficher ou masquer la sélection",
          Scan,
          () =>
            document
              .querySelector(".app")
              ?.classList.toggle("selection-hidden"),
          disabled,
        )}
        {button("Insérer un texte", Type, () => p.tool("text"), disabled)}
        {button("Tracé vectoriel", PenTool, () => p.tool("pen"), disabled)}
        {sep}
        {button("Explorateur d’images — indisponible", Folder, () => {}, true)}
        {button("Automatisation", Workflow, () => {
          const d =
            document.querySelector<HTMLDetailsElement>(".actions-panel");
          if (d) {
            d.open = true;
            d.scrollIntoView({ block: "nearest" });
          }
        })}
        {button("Thème clair / sombre", Settings, () =>
          document
            .querySelector<HTMLButtonElement>('[aria-label="Changer de thème"]')
            ?.click(),
        )}
        {sep}
        <select
          aria-label="Zoom d’affichage"
          value={Math.round(p.zoom * 100)}
          onChange={(e) =>
            e.target.value === "fit"
              ? p.fit()
              : p.setZoom(+e.target.value / 100)
          }
        >
          {![5, 10, 25, 50, 75, 100, 150, 200, 400, 800, 1600].includes(
            Math.round(p.zoom * 100),
          ) && (
            <option value={Math.round(p.zoom * 100)}>
              {Math.round(p.zoom * 100)} %
            </option>
          )}
          {[5, 10, 25, 50, 75, 100, 150, 200, 400, 800, 1600].map((n) => (
            <option key={n} value={n}>
              {n} %
            </option>
          ))}
          <option value="fit">Adapter</option>
        </select>
        {button(
          "Zoom avant",
          ZoomIn,
          () => p.setZoom(Math.min(16, p.zoom * 1.25)),
          disabled,
        )}
        {button(
          "Zoom arrière",
          ZoomOut,
          () => p.setZoom(Math.max(0.05, p.zoom / 1.25)),
          disabled,
        )}
        {button(
          "Taille réelle (Ctrl 1)",
          Maximize,
          () => p.setZoom(1),
          disabled,
        )}
        {button("Adapter (Ctrl 0)", Fullscreen, p.fit, disabled)}
      </div>
      <div className="filterbar" aria-label="Réglages et filtres rapides">
        {button(
          "Luminosité −",
          Sun,
          adjustment("brightness", { brightness: -10, contrast: 0 }),
          raster,
          "−",
        )}
        {button(
          "Luminosité +",
          Sun,
          adjustment("brightness", { brightness: 10, contrast: 0 }),
          raster,
          "+",
        )}
        {sep}
        {button(
          "Contraste −",
          Contrast,
          adjustment("brightness", { brightness: 0, contrast: -10 }),
          raster,
          "−",
        )}
        {button(
          "Contraste +",
          Contrast,
          adjustment("brightness", { brightness: 0, contrast: 10 }),
          raster,
          "+",
        )}
        {sep}
        {button(
          "Gamma −",
          Gamma,
          adjustment("gamma", { gamma: 0.9 }),
          raster,
          "−",
        )}
        {button(
          "Gamma +",
          Gamma,
          adjustment("gamma", { gamma: 1.1 }),
          raster,
          "+",
        )}
        {sep}
        {button(
          "Saturation −",
          Palette,
          adjustment("hsl", { hue: 0, saturation: -10, lightness: 0 }),
          raster,
          "−",
        )}
        {button(
          "Saturation +",
          Palette,
          adjustment("hsl", { hue: 0, saturation: 10, lightness: 0 }),
          raster,
          "+",
        )}
        {sep}
        {button(
          "Histogramme et courbes",
          ChartNoAxesColumn,
          () => p.filter("curves"),
          raster,
        )}
        {button(
          "Réglage personnalisé",
          SlidersHorizontal,
          () => p.filter("brightness"),
          raster,
        )}
        {button(
          "Balance des couleurs",
          Blend,
          () => p.filter("balance"),
          raster,
        )}
        {sep}
        {button(
          "Niveaux automatiques",
          Wand,
          () => p.filter("auto-levels"),
          raster,
        )}
        {button(
          "Contraste automatique",
          Sparkles,
          () => p.filter("auto-contrast"),
          raster,
        )}
        {sep}
        {button(
          "Niveaux de gris",
          Palette,
          () => p.filter("grayscale"),
          raster,
        )}
        {button("Sépia", Palette, () => p.filter("sepia"), raster)}
        {button(
          "Photographie ancienne",
          Image,
          () => p.filter("old-photo"),
          raster,
        )}
        {sep}
        {button("Antipoussière", Paintbrush, () => p.filter("median"), raster)}
        {button("Adoucir", Droplets, adjustment("blur", { radius: 1 }), raster)}
        {button("Flou", Droplets, () => p.filter("blur"), raster)}
        {sep}
        {button("Relief", Triangle, () => p.filter("emboss"), raster)}
        {button("Plus net", Focus, () => p.filter("sharpen"), raster)}
        {button("Renforcement", Focus, () => p.filter("unsharp"), raster)}
        {sep}
        {button("Variation de teinte", Palette, () => p.filter("hsl"), raster)}
        {button("Dégradé", Blend, () => p.tool("gradient"), disabled)}
        {button(
          "Masque",
          Layers,
          () =>
            p.dialog({
              kind: "asset",
              title: "Masque",
              values: {},
              asset: assetDefaults("mask", p.color, "#ffffff"),
            }),
          raster,
        )}
        {sep}
        {button(
          "Miroir vertical",
          MoveVertical,
          command({ type: "flip", horizontal: false }),
          disabled,
        )}
        {button(
          "Miroir horizontal",
          MoveHorizontal,
          command({ type: "flip", horizontal: true }),
          disabled,
        )}
        {button(
          "Rotation 90° gauche",
          RotateCcw,
          command({ type: "rotate", degrees: 270 }),
          disabled,
        )}
        {button(
          "Rotation 90° droite",
          RotateCw,
          command({ type: "rotate", degrees: 90 }),
          disabled,
        )}
      </div>
    </div>
  );
}

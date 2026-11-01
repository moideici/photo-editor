export interface Parameter {
  key: string;
  label: string;
  min: number;
  max: number;
  step?: number;
  value: number;
}
export interface Filter {
  id: string;
  label: string;
  group: "Réglages" | "Filtres";
  params: Parameter[];
  editor?: "curves";
  colors?: { key: string; label: string; value: string }[];
  checks?: { key: string; label: string; value: boolean }[];
}
const p = (
  key: string,
  label: string,
  min: number,
  max: number,
  value: number,
  step = 1,
): Parameter => ({ key, label, min, max, value, step });
export const filters: Filter[] = [
  {
    id: "auto-contrast",
    label: "Contraste automatique",
    group: "Réglages",
    params: [],
  },
  {
    id: "old-photo",
    label: "Photographie ancienne",
    group: "Filtres",
    params: [],
  },
  {
    id: "curves",
    label: "Courbes…",
    group: "Réglages",
    params: [],
    editor: "curves",
  },
  {
    id: "levels",
    label: "Niveaux manuels",
    group: "Réglages",
    params: [
      p("inputBlack", "Noir d’entrée", 0, 254, 0),
      p("inputWhite", "Blanc d’entrée", 1, 255, 255),
      p("gamma", "Gamma", 0.1, 10, 1, 0.1),
      p("outputBlack", "Noir de sortie", 0, 255, 0),
      p("outputWhite", "Blanc de sortie", 0, 255, 255),
    ],
  },
  {
    id: "replace-color",
    label: "Remplacement de couleur",
    group: "Réglages",
    params: [
      p("tolerance", "Tolérance", 0, 255, 30),
      p("softness", "Transition", 0, 255, 20),
      p("amount", "Intensité (%)", 0, 100, 100),
    ],
    colors: [
      { key: "sourceColor", label: "Couleur à remplacer", value: "#ff0000" },
      { key: "targetColor", label: "Nouvelle couleur", value: "#0000ff" },
    ],
    checks: [
      { key: "preserveShading", label: "Conserver les nuances", value: true },
    ],
  },
  {
    id: "channel-mixer",
    label: "Mélangeur de couches",
    group: "Réglages",
    params: ["red", "green", "blue"].flatMap((label, row) =>
      ["Red", "Green", "Blue", "Constant"].map((input, column) =>
        p(
          `${label}${input}`,
          `${["Rouge", "Vert", "Bleu"][row]} ← ${["rouge", "vert", "bleu", "constante"][column]} (%)`,
          -200,
          200,
          row === column ? 100 : 0,
        ),
      ),
    ),
    checks: [
      { key: "monochrome", label: "Monochrome", value: false },
      {
        key: "preserveLuminosity",
        label: "Préserver la luminosité",
        value: false,
      },
    ],
  },
  {
    id: "selective-saturation",
    label: "Désaturation sélective",
    group: "Réglages",
    params: [
      p("hue", "Teinte cible (°)", 0, 360, 0),
      p("range", "Étendue (°)", 0, 360, 60),
      p("softness", "Transition (°)", 0, 180, 30),
      p("saturation", "Saturation (%)", -100, 100, -100),
    ],
    checks: [
      { key: "invert", label: "Agir sur les autres teintes", value: false },
    ],
  },
  {
    id: "brightness",
    label: "Luminosité / Contraste",
    group: "Réglages",
    params: [
      p("brightness", "Luminosité", -100, 100, 0),
      p("contrast", "Contraste", -100, 100, 0),
    ],
  },
  {
    id: "hsl",
    label: "Teinte / Saturation",
    group: "Réglages",
    params: [
      p("hue", "Teinte", -180, 180, 0),
      p("saturation", "Saturation", -100, 100, 0),
      p("lightness", "Luminosité", -100, 100, 0),
    ],
  },
  {
    id: "gamma",
    label: "Correction gamma",
    group: "Réglages",
    params: [p("gamma", "Gamma", 0.1, 5, 1, 0.1)],
  },
  {
    id: "balance",
    label: "Balance des couleurs",
    group: "Réglages",
    params: [
      p("red", "Rouge", -255, 255, 0),
      p("green", "Vert", -255, 255, 0),
      p("blue", "Bleu", -255, 255, 0),
    ],
  },
  {
    id: "auto-levels",
    label: "Niveaux automatiques",
    group: "Réglages",
    params: [],
  },
  {
    id: "threshold",
    label: "Seuil",
    group: "Réglages",
    params: [p("threshold", "Seuil", 0, 255, 128)],
  },
  {
    id: "posterize",
    label: "Postérisation",
    group: "Réglages",
    params: [p("levels", "Niveaux", 2, 32, 6)],
  },
  { id: "invert", label: "Négatif", group: "Réglages", params: [] },
  { id: "grayscale", label: "Niveaux de gris", group: "Réglages", params: [] },
  { id: "sepia", label: "Sépia", group: "Réglages", params: [] },
  {
    id: "blur",
    label: "Flou gaussien",
    group: "Filtres",
    params: [p("radius", "Rayon", 0.3, 50, 3, 0.1)],
  },
  {
    id: "motion",
    label: "Flou de mouvement",
    group: "Filtres",
    params: [
      p("angle", "Angle", -180, 180, 0),
      p("distance", "Distance", 1, 150, 15),
    ],
  },
  { id: "sharpen", label: "Netteté", group: "Filtres", params: [] },
  {
    id: "unsharp",
    label: "Masque flou",
    group: "Filtres",
    params: [
      p("radius", "Rayon", 0.3, 30, 2, 0.1),
      p("amount", "Intensité", 0, 5, 1.5, 0.1),
      p("threshold", "Seuil", 0, 100, 3),
    ],
  },
  { id: "emboss", label: "Relief", group: "Filtres", params: [] },
  { id: "edges", label: "Contours", group: "Filtres", params: [] },
  {
    id: "pixelate",
    label: "Pixellisation",
    group: "Filtres",
    params: [p("size", "Taille", 2, 100, 10)],
  },
  {
    id: "noise",
    label: "Bruit",
    group: "Filtres",
    params: [p("amount", "Intensité", 0, 100, 15)],
  },
  {
    id: "median",
    label: "Médiane",
    group: "Filtres",
    params: [p("radius", "Rayon", 1, 5, 2)],
  },
  {
    id: "vignette",
    label: "Vignettage",
    group: "Filtres",
    params: [
      p("strength", "Intensité", 0, 1, 0.7, 0.05),
      p("radius", "Zone préservée", 0, 0.95, 0.3, 0.05),
    ],
  },
  {
    id: "oil",
    label: "Peinture à l’huile",
    group: "Filtres",
    params: [p("radius", "Rayon", 1, 8, 3), p("levels", "Niveaux", 4, 64, 16)],
  },
  {
    id: "crystallize",
    label: "Cristallisation",
    group: "Filtres",
    params: [p("size", "Taille", 2, 100, 20)],
  },
];

const cherryIcons = import.meta.glob<string>("../assets/provider-icons/cherry/*.svg", {
  eager: true,
  import: "default",
  query: "?url",
});
const cherryDarkIcons = import.meta.glob<string>("../assets/provider-icons/cherry/dark/*.svg", {
  eager: true,
  import: "default",
  query: "?url",
});

const iconAliases: Record<string, string> = {
  siliconflow: "silicon",
  xai: "grok",
  stepfun: "step",
  xiaomi: "xiaomimimo",
};

const providerMarks: Record<string, string> = {
  anthropic: "A", openai: "O", openrouter: "OR", deepseek: "DS", zhipu: "GLM",
  moonshot: "K", siliconflow: "SF", bailian: "Q", volcengine: "V", groq: "G",
  xai: "x", mistral: "M", together: "T", fireworks: "F", nvidia: "N",
  cerebras: "C", minimax: "MM", stepfun: "S", baichuan: "B", ppio: "P",
  qiniu: "7N", modelscope: "MS", xiaomi: "MI", zai: "Z",
};

const fallbackMark = "AI";

function resolveProviderIcon(iconKey: string) {
  const fileName = iconAliases[iconKey] ?? iconKey;
  return {
    iconUrl: cherryIcons[`../assets/provider-icons/cherry/${fileName}.svg`],
    darkIconUrl: cherryDarkIcons[`../assets/provider-icons/cherry/dark/${fileName}.svg`],
  };
}

export function hasBundledProviderIcon(iconKey: string): boolean {
  return Boolean(resolveProviderIcon(iconKey).iconUrl);
}

export function ProviderBrandIcon({ iconKey, label, size = "md" }: {
  iconKey: string;
  label: string;
  size?: "sm" | "md" | "lg";
}) {
  const mark = providerMarks[iconKey] ?? fallbackMark;
  const { iconUrl, darkIconUrl } = resolveProviderIcon(iconKey);
  const sizeClasses = size === "sm" ? "h-8 w-8 text-[9px]" : size === "lg" ? "h-12 w-12 text-xs" : "h-10 w-10 text-[10px]";

  return (
    <span aria-label={label} className={`inline-flex shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border bg-white p-1.5 font-bold tracking-tight text-muted-foreground shadow-xs dark:bg-zinc-950 ${sizeClasses}`} role="img">
      {!iconUrl ? mark : <>
        <img alt="" aria-hidden="true" className={`h-full w-full object-contain ${darkIconUrl ? "dark:hidden" : ""}`} src={iconUrl} />
        {darkIconUrl ? <img alt="" aria-hidden="true" className="hidden h-full w-full object-contain dark:block" src={darkIconUrl} /> : null}
      </>}
    </span>
  );
}

import { memo, type ReactNode } from "react";
import ReactMarkdown, { defaultUrlTransform, type Components, type UrlTransform } from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import { cn } from "../../lib/utils";
import { MermaidDiagram } from "./MermaidDiagram";
import { SafeImage, safeImageSource } from "./SafeImage";

const components: Components = {
  a({ children, ...props }) {
    return <a className="break-words text-primary underline underline-offset-4" rel="noreferrer" target="_blank" {...props}>{children}</a>;
  },
  blockquote({ children, ...props }) {
    return <blockquote className="my-3 border-l-4 border-border pl-3 text-muted-foreground" {...props}>{children}</blockquote>;
  },
  code({ className, children, ...props }) {
    const content = String(children).replace(/\n$/, "");
    if (/\blanguage-mermaid\b/.test(className ?? "")) return <MermaidDiagram chart={content} />;
    return <code className={className} {...props}>{children}</code>;
  },
  h1({ children, ...props }) {
    return <h1 className="mb-2 mt-4 text-xl font-semibold first:mt-0" {...props}>{children}</h1>;
  },
  h2({ children, ...props }) {
    return <h2 className="mb-2 mt-4 text-lg font-semibold first:mt-0" {...props}>{children}</h2>;
  },
  h3({ children, ...props }) {
    return <h3 className="mb-1.5 mt-3 text-base font-semibold first:mt-0" {...props}>{children}</h3>;
  },
  img({ src, alt }) {
    return <SafeImage alt={alt ?? undefined} src={src} />;
  },
  li({ children, ...props }) {
    return <li className="my-1 pl-0.5" {...props}>{children}</li>;
  },
  ol({ children, ...props }) {
    return <ol className="my-2 list-decimal space-y-1 pl-6" {...props}>{children}</ol>;
  },
  p({ children, ...props }) {
    return <p className="my-2 first:mt-0 last:mb-0" {...props}>{children}</p>;
  },
  pre({ children, ...props }) {
    return <pre className="my-3 max-w-full overflow-x-auto rounded-md border border-border bg-muted p-3 text-xs leading-5" {...props}>{children}</pre>;
  },
  table({ children, ...props }) {
    return <div className="my-3 max-w-full overflow-x-auto rounded-md border border-border"><table className="w-full border-collapse text-left text-sm" {...props}>{children}</table></div>;
  },
  td({ children, ...props }) {
    return <td className="border-t border-border px-3 py-2 align-top" {...props}>{children}</td>;
  },
  th({ children, ...props }) {
    return <th className="bg-muted px-3 py-2 font-semibold" {...props}>{children}</th>;
  },
  ul({ children, className, ...props }) {
    return <ul className={cn("my-2 list-disc space-y-1 pl-6", className?.includes("contains-task-list") && "list-none pl-1")} {...props}>{children}</ul>;
  },
};

const urlTransform: UrlTransform = (url, key, node) => {
  if (node.tagName === "img" && key === "src") return safeImageSource(url);
  return defaultUrlTransform(url);
};

/**
 * The one safe Markdown renderer.
 *
 * Links open externally with `noreferrer`, images pass through `safeImageSource`, code is
 * highlighted, math goes through KaTeX, and `mermaid` fences become diagrams. Raw HTML is inert
 * because `rehype-raw` is deliberately absent — react-markdown does not parse it, so a `<script>`
 * in a document is text.
 *
 * `headingIds` assigns ids by position for callers that already derived an outline from the same
 * source. By position rather than from the rendered text: the renderer sees children after parsing,
 * where emphasis and links are already elements, so re-deriving here would be a second parser that
 * disagrees with the first exactly on headings containing markup.
 *
 * Memoized (task 10.13): re-parsing Markdown/KaTeX/Mermaid on every token of an unrelated
 * streaming sibling is exactly the "expensive... renderer" cost this task names avoiding.
 */
export const RichMarkdown = memo(function RichMarkdown({
  children,
  className,
  headingIds,
}: {
  children: string;
  className?: string;
  headingIds?: readonly string[];
}) {
  return (
    <div className={cn("rich-markdown min-w-0 max-w-none wrap-break-word whitespace-normal leading-6 text-inherit", className)}>
      <ReactMarkdown
        components={headingIds ? withHeadingIds(components, headingIds) : components}
        rehypePlugins={[rehypeKatex, [rehypeHighlight, { detect: false, ignoreMissing: true }]]}
        remarkPlugins={[remarkGfm, remarkMath]}
        urlTransform={urlTransform}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
});

/**
 * The same components, with an id on each heading.
 *
 * A fresh counter per call, because the renderer walks the document once per render and an id
 * assigned from a shared count would drift by one on every re-render.
 */
function withHeadingIds(base: Components, ids: readonly string[]): Components {
  let consumed = 0;
  const decorate = (tag: "h1" | "h2" | "h3" | "h4" | "h5" | "h6") =>
    function Heading({ children, ...props }: { children?: ReactNode }) {
      const id = ids[consumed];
      consumed += 1;
      const Base = base[tag];
      const rendered = Base ? <Base {...props}>{children}</Base> : <>{children}</>;
      // Wrapped rather than passed down: the base components spread their own props onto the
      // heading, and an id threaded through them would be one more thing each has to remember.
      return <div id={id}>{rendered}</div>;
    };
  return {
    ...base,
    h1: decorate("h1"),
    h2: decorate("h2"),
    h3: decorate("h3"),
    h4: decorate("h4"),
    h5: decorate("h5"),
    h6: decorate("h6"),
  };
}

import { type ReactNode, useEffect, useState } from "react";
import { codeToHtml } from "shiki";
import CopyButton from "../copy-button";

interface CodeHighlightProps {
  className?: string;
  children?: ReactNode;
}

const CodeHighlight = ({ className, children, ...props }: CodeHighlightProps) => {
  const code = String(children).trim();
  const language = className?.match(/language-(\w+)/)?.[1] || "text";
  const isInline = !className;

  const [html, setHtml] = useState<string>("");

  useEffect(() => {
    if (isInline) return;
    let cancelled = false;
    codeToHtml(code, {
      lang: language,
      themes: { light: "github-light", dark: "github-dark" },
    }).then((result) => {
      if (!cancelled) setHtml(result);
    });
    return () => { cancelled = true; };
  }, [code, language, isInline]);

  if (isInline) {
    return (
      <code className="rounded bg-muted px-1.5 py-0.5 text-[13px] font-mono" {...props}>
        {children}
      </code>
    );
  }

  return (
    <div className="relative group">
      <div className="shiki-header">
        <span className="language-label text-xs text-muted-foreground">{language}</span>
      </div>
      <div className="shiki-code">
        {html ? (
          <div dangerouslySetInnerHTML={{ __html: html }} />
        ) : (
          <pre className={className} {...props}>
            <code>{children}</code>
          </pre>
        )}
      </div>
      <CopyButton text={code} />
    </div>
  );
};

export default CodeHighlight;

/**
 * TipTap rich text editor with / slash-commands and @ mentions.
 */
import { cn } from "@/lib/utils";
import { BUILTIN_PERSONAS } from "@/lib/builtin-personas";
import Mention from "@tiptap/extension-mention";
import Placeholder from "@tiptap/extension-placeholder";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import {
  Sparkles,
  User,
  Wrench,
} from "lucide-react";
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
} from "react";
import { SlashCommand } from "./slash-command";
import { createSuggestionRenderer } from "./suggestion-renderer";
import type { SuggestionItem } from "./mention-list";
import "./tiptap.css";

// =============================================================================
// Data sources for / and @
// =============================================================================

/** Skills available for /slash-command */
const SLASH_ITEMS: SuggestionItem[] = [
  { id: "factor_analysis", label: "factor_analysis", description: "批量因子检验 — IC、分层回测、归因", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "model_monitor", label: "model_monitor", description: "模型监控 — PSI、AUC 衰减、漂移告警", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "k8s_upgrade_preflight", label: "k8s_upgrade_preflight", description: "K8s 升级预检自动化", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "pipeline_quality_monitor", label: "pipeline_quality_monitor", description: "数据管道质量监控", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "competitive_intel", label: "competitive_intel", description: "竞品情报自动采集与分析", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "backtest_report", label: "backtest_report", description: "策略回测报告生成", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "credit_feature_eng", label: "credit_feature_eng", description: "信用特征工程自动化", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "schema_migration", label: "schema_migration", description: "数据库 Schema 迁移管理", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "incident_runbook", label: "incident_runbook", description: "故障应急 Runbook 执行", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "prd_template", label: "prd_template", description: "PRD 模板生成与校验", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "payment_approval", label: "payment_approval", description: "供应商付款审批与执行", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "invoice_reconcile", label: "invoice_reconcile", description: "发票自动核对与对账", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  { id: "ab_test_analyzer", label: "ab_test_analyzer", description: "A/B 测试显著性分析", group: "技能", icon: <Sparkles className="size-3 text-primary" /> },
  // Tools
  { id: "tool_read", label: "Read", description: "读取文件内容", group: "工具", icon: <Wrench className="size-3 text-muted-foreground" /> },
  { id: "tool_write", label: "Write", description: "写入文件", group: "工具", icon: <Wrench className="size-3 text-muted-foreground" /> },
  { id: "tool_bash", label: "Bash", description: "执行终端命令", group: "工具", icon: <Wrench className="size-3 text-muted-foreground" /> },
  { id: "tool_web_search", label: "WebSearch", description: "联网搜索", group: "工具", icon: <Wrench className="size-3 text-muted-foreground" /> },
  { id: "tool_python", label: "PythonExec", description: "执行 Python 代码", group: "工具", icon: <Wrench className="size-3 text-muted-foreground" /> },
  { id: "tool_sql", label: "SQLExecute", description: "执行 SQL 查询", group: "工具", icon: <Wrench className="size-3 text-muted-foreground" /> },
];

/** Items available for @mention: agents */
const MENTION_ITEMS: SuggestionItem[] = [
  // Agents
  ...BUILTIN_PERSONAS.filter((p) => p.id !== "company-group").map((p) => ({
    id: p.id,
    label: p.name,
    description: p.description,
    group: "智能体",
    icon: <User className="size-3 text-primary" />,
  })),
];

function filterItems(items: SuggestionItem[], query: string): SuggestionItem[] {
  const q = query.toLowerCase();
  if (!q) return items.slice(0, 15);
  return items
    .filter(
      (item) =>
        item.label.toLowerCase().includes(q) ||
        item.id.toLowerCase().includes(q) ||
        item.description?.toLowerCase().includes(q),
    )
    .slice(0, 12);
}

// =============================================================================
// Editor component
// =============================================================================

export interface TiptapEditorRef {
  focus: () => void;
  getText: () => string;
  clear: () => void;
  isEmpty: () => boolean;
}

interface TiptapEditorProps {
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  onSubmit?: (text: string) => void;
  onChange?: (text: string) => void;
}

const TiptapEditor = forwardRef<TiptapEditorRef, TiptapEditorProps>(
  ({ placeholder, disabled, className, onSubmit, onChange }, ref) => {
    const slashSuggestion = useMemo(
      () => createSuggestionRenderer((q) => filterItems(SLASH_ITEMS, q)),
      [],
    );

    const mentionSuggestion = useMemo(
      () => createSuggestionRenderer((q) => filterItems(MENTION_ITEMS, q)),
      [],
    );

    const editor = useEditor({
      extensions: [
        StarterKit.configure({
          // Disable block-level features — this is a chat input, not a document editor
          heading: false,
          blockquote: false,
          codeBlock: false,
          horizontalRule: false,
          bulletList: false,
          orderedList: false,
          listItem: false,
        }),
        Placeholder.configure({
          placeholder: placeholder || "输入消息...",
          emptyEditorClass: "tiptap-empty",
        }),
        Mention.configure({
          HTMLAttributes: {
            class: "tiptap-mention",
          },
          renderHTML({ options, node }) {
            return [
              "span",
              options.HTMLAttributes,
              `@${node.attrs.label ?? node.attrs.id}`,
            ];
          },
          suggestion: {
            char: "@",
            ...mentionSuggestion,
          },
        }),
        SlashCommand.configure({
          suggestion: {
            ...slashSuggestion,
          },
        }),
      ],
      editable: !disabled,
      editorProps: {
        attributes: {
          class: "tiptap-content",
        },
        handleKeyDown: (_view, event) => {
          // Enter without Shift = submit
          if (event.key === "Enter" && !event.shiftKey) {
            // Don't submit if suggestion popup is open — suggestion handles it
            // The suggestion plugin returns true for Enter when it's open, so
            // this only fires when no suggestion is active.
            event.preventDefault();
            const text = editor?.getText().trim();
            if (text) {
              onSubmit?.(text);
              // Clear after a microtask to avoid interfering with ProseMirror
              setTimeout(() => editor?.commands.clearContent(), 0);
            }
            return true;
          }
          return false;
        },
      },
      onUpdate: ({ editor: e }) => {
        onChange?.(e.getText());
      },
    });

    // Sync disabled state
    useEffect(() => {
      if (editor) {
        editor.setEditable(!disabled);
      }
    }, [editor, disabled]);

    useImperativeHandle(
      ref,
      () => ({
        focus: () => editor?.commands.focus(),
        getText: () => editor?.getText() || "",
        clear: () => editor?.commands.clearContent(),
        isEmpty: () => editor?.isEmpty ?? true,
      }),
      [editor],
    );

    const handleContainerClick = useCallback(() => {
      editor?.commands.focus();
    }, [editor]);

    return (
      <div
        className={cn(
          "w-full h-full overflow-y-auto cursor-text",
          className,
        )}
        onClick={handleContainerClick}
      >
        <EditorContent editor={editor} className="h-full" />
      </div>
    );
  },
);

TiptapEditor.displayName = "TiptapEditor";

export default TiptapEditor;

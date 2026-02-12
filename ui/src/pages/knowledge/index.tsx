import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { useMemo, useState } from "react";
import {
  BookOpen,
  ChevronDown,
  ChevronRight,
  Clock,
  Download,
  Eye,
  File,
  FileCode,
  FileSpreadsheet,
  FileText,
  FileType,
  Folder,
  FolderOpen,
  Grid3X3,
  HardDrive,
  List,
  MoreHorizontal,
  Pencil,
  Plus,
  Search,
  Star,
  Trash2,
  Upload,
} from "lucide-react";

// =============================================================================
// Types
// =============================================================================

interface KnowledgeFile {
  id: string;
  name: string;
  type: "folder" | "docx" | "xlsx" | "pptx" | "pdf" | "md" | "txt" | "csv" | "json" | "html";
  size?: number;
  updatedAt: number;
  updatedBy?: string;
  starred?: boolean;
  children?: KnowledgeFile[];
  tags?: string[];
}

type ViewMode = "list" | "grid";
type SortBy = "name" | "updated" | "size";

// =============================================================================
// File icon mapping
// =============================================================================

const FILE_ICONS: Record<string, { icon: typeof File; color: string }> = {
  folder: { icon: Folder, color: "text-primary" },
  docx: { icon: FileText, color: "text-blue-500" },
  xlsx: { icon: FileSpreadsheet, color: "text-green-600 dark:text-green-400" },
  pptx: { icon: FileType, color: "text-orange-500" },
  pdf: { icon: FileText, color: "text-red-500" },
  md: { icon: FileCode, color: "text-purple-500" },
  txt: { icon: File, color: "text-muted-foreground" },
  csv: { icon: FileSpreadsheet, color: "text-green-600 dark:text-green-400" },
  json: { icon: FileCode, color: "text-yellow-600 dark:text-yellow-400" },
  html: { icon: FileCode, color: "text-orange-500" },
};

function FileIcon({ type, className }: { type: string; className?: string }) {
  const config = FILE_ICONS[type] || { icon: File, color: "text-muted-foreground" };
  const Icon = config.icon;
  return <Icon className={cn("size-4", config.color, className)} />;
}

function FolderIcon({ open, className }: { open?: boolean; className?: string }) {
  const Icon = open ? FolderOpen : Folder;
  return <Icon className={cn("size-4 text-primary", className)} />;
}

// =============================================================================
// Mock knowledge base data
// =============================================================================

const now = Date.now();

const MOCK_KNOWLEDGE: KnowledgeFile[] = [
  {
    id: "kb-1",
    name: "合规与风控",
    type: "folder",
    updatedAt: now - 2 * 3600_000,
    children: [
      {
        id: "kb-1-1",
        name: "反洗钱政策",
        type: "folder",
        updatedAt: now - 4 * 3600_000,
        children: [
          { id: "kb-1-1-1", name: "AML_KYC_流程手册_v3.2.docx", type: "docx", size: 2_450_000, updatedAt: now - 4 * 3600_000, updatedBy: "风控分析师", tags: ["AML", "KYC"] },
          { id: "kb-1-1-2", name: "可疑交易监控规则.xlsx", type: "xlsx", size: 890_000, updatedAt: now - 12 * 3600_000, updatedBy: "风控分析师", tags: ["监控", "规则"] },
          { id: "kb-1-1-3", name: "FATF旅行规则解读.pdf", type: "pdf", size: 5_600_000, updatedAt: now - 3 * 86400_000, updatedBy: "风控分析师", tags: ["FATF", "跨境"] },
        ],
      },
      {
        id: "kb-1-2",
        name: "数据隐私",
        type: "folder",
        updatedAt: now - 8 * 3600_000,
        children: [
          { id: "kb-1-2-1", name: "个人信息保护合规检查清单.md", type: "md", size: 45_000, updatedAt: now - 8 * 3600_000, updatedBy: "风控分析师", starred: true, tags: ["隐私", "PIPL"] },
          { id: "kb-1-2-2", name: "GDPR_与_PIPL_对比分析.docx", type: "docx", size: 1_200_000, updatedAt: now - 5 * 86400_000, updatedBy: "产品经理" },
          { id: "kb-1-2-3", name: "数据分类分级标准.pdf", type: "pdf", size: 3_400_000, updatedAt: now - 7 * 86400_000, updatedBy: "风控分析师" },
        ],
      },
      { id: "kb-1-3", name: "各国跨境支付监管要求汇总.xlsx", type: "xlsx", size: 3_200_000, updatedAt: now - 2 * 86400_000, updatedBy: "产品经理", starred: true, tags: ["跨境", "监管"] },
      { id: "kb-1-4", name: "风控模型评估报告_2025Q1.pdf", type: "pdf", size: 8_900_000, updatedAt: now - 2 * 3600_000, updatedBy: "风控分析师" },
    ],
  },
  {
    id: "kb-2",
    name: "产品文档",
    type: "folder",
    updatedAt: now - 6 * 3600_000,
    children: [
      {
        id: "kb-2-1",
        name: "产品需求文档",
        type: "folder",
        updatedAt: now - 6 * 3600_000,
        children: [
          { id: "kb-2-1-1", name: "跨境支付_PRD_v2.1.md", type: "md", size: 125_000, updatedAt: now - 6 * 3600_000, updatedBy: "产品经理", starred: true, tags: ["PRD", "跨境"] },
          { id: "kb-2-1-2", name: "AI智能对账_PRD_v1.0.md", type: "md", size: 89_000, updatedAt: now - 3 * 86400_000, updatedBy: "产品经理", tags: ["PRD", "AI"] },
          { id: "kb-2-1-3", name: "数字钱包_产品规划.pptx", type: "pptx", size: 15_400_000, updatedAt: now - 10 * 86400_000, updatedBy: "产品经理" },
        ],
      },
      {
        id: "kb-2-2",
        name: "竞品分析",
        type: "folder",
        updatedAt: now - 24 * 3600_000,
        children: [
          { id: "kb-2-2-1", name: "Airwallex_竞品分析_2025.md", type: "md", size: 67_000, updatedAt: now - 24 * 3600_000, updatedBy: "产品经理", tags: ["竞品"] },
          { id: "kb-2-2-2", name: "Stripe_Connect_功能对比.xlsx", type: "xlsx", size: 450_000, updatedAt: now - 5 * 86400_000, updatedBy: "产品经理" },
          { id: "kb-2-2-3", name: "东南亚支付市场报告.pdf", type: "pdf", size: 12_000_000, updatedAt: now - 15 * 86400_000, updatedBy: "财务分析师" },
        ],
      },
      { id: "kb-2-3", name: "API接口文档_v3.4.html", type: "html", size: 340_000, updatedAt: now - 48 * 3600_000, updatedBy: "全栈工程师" },
    ],
  },
  {
    id: "kb-3",
    name: "数据与模型",
    type: "folder",
    updatedAt: now - 3 * 3600_000,
    children: [
      {
        id: "kb-3-1",
        name: "模型文档",
        type: "folder",
        updatedAt: now - 3 * 3600_000,
        children: [
          { id: "kb-3-1-1", name: "信用评分模型_v3_说明书.md", type: "md", size: 78_000, updatedAt: now - 3 * 3600_000, updatedBy: "数据科学家", starred: true, tags: ["模型", "信用评分"] },
          { id: "kb-3-1-2", name: "模型监控指标定义.md", type: "md", size: 34_000, updatedAt: now - 2 * 86400_000, updatedBy: "数据科学家" },
          { id: "kb-3-1-3", name: "特征工程手册.docx", type: "docx", size: 980_000, updatedAt: now - 7 * 86400_000, updatedBy: "数据科学家" },
        ],
      },
      {
        id: "kb-3-2",
        name: "数据字典",
        type: "folder",
        updatedAt: now - 12 * 3600_000,
        children: [
          { id: "kb-3-2-1", name: "交易数据字典_v2.xlsx", type: "xlsx", size: 560_000, updatedAt: now - 12 * 3600_000, updatedBy: "数据工程师" },
          { id: "kb-3-2-2", name: "用户画像字段说明.csv", type: "csv", size: 120_000, updatedAt: now - 4 * 86400_000, updatedBy: "数据工程师" },
        ],
      },
      { id: "kb-3-3", name: "数据管道架构图.md", type: "md", size: 56_000, updatedAt: now - 48 * 3600_000, updatedBy: "数据工程师" },
      { id: "kb-3-4", name: "ClickHouse_表结构.json", type: "json", size: 23_000, updatedAt: now - 5 * 86400_000, updatedBy: "数据工程师" },
    ],
  },
  {
    id: "kb-4",
    name: "运维与基础设施",
    type: "folder",
    updatedAt: now - 18 * 3600_000,
    children: [
      { id: "kb-4-1", name: "K8s集群运维手册.md", type: "md", size: 145_000, updatedAt: now - 18 * 3600_000, updatedBy: "运维工程师", tags: ["K8s", "运维"] },
      { id: "kb-4-2", name: "监控告警规则配置.json", type: "json", size: 67_000, updatedAt: now - 2 * 86400_000, updatedBy: "运维工程师" },
      { id: "kb-4-3", name: "灾备切换流程.docx", type: "docx", size: 890_000, updatedAt: now - 10 * 86400_000, updatedBy: "运维工程师" },
      { id: "kb-4-4", name: "服务器资源清单.xlsx", type: "xlsx", size: 230_000, updatedAt: now - 3 * 86400_000, updatedBy: "运维工程师" },
    ],
  },
  {
    id: "kb-5",
    name: "财务与报告",
    type: "folder",
    updatedAt: now - 5 * 3600_000,
    children: [
      { id: "kb-5-1", name: "2025Q1_财务报表.xlsx", type: "xlsx", size: 4_500_000, updatedAt: now - 5 * 3600_000, updatedBy: "财务分析师", starred: true },
      { id: "kb-5-2", name: "资金流动性分析_202501.pdf", type: "pdf", size: 6_700_000, updatedAt: now - 3 * 86400_000, updatedBy: "财务分析师" },
      { id: "kb-5-3", name: "汇率风险对冲方案.docx", type: "docx", size: 1_100_000, updatedAt: now - 7 * 86400_000, updatedBy: "财务分析师" },
      { id: "kb-5-4", name: "跨境支付手续费分析.csv", type: "csv", size: 780_000, updatedAt: now - 14 * 86400_000, updatedBy: "财务分析师" },
    ],
  },
  {
    id: "kb-6",
    name: "团队知识沉淀",
    type: "folder",
    updatedAt: now - 24 * 3600_000,
    children: [
      { id: "kb-6-1", name: "新人入职指南.md", type: "md", size: 56_000, updatedAt: now - 24 * 3600_000, updatedBy: "产品经理" },
      { id: "kb-6-2", name: "技术选型决策记录.md", type: "md", size: 89_000, updatedAt: now - 5 * 86400_000, updatedBy: "全栈工程师" },
      { id: "kb-6-3", name: "复盘_跨境支付延迟事件_20250115.md", type: "md", size: 34_000, updatedAt: now - 20 * 86400_000, updatedBy: "运维工程师" },
    ],
  },
];

// =============================================================================
// Helpers
// =============================================================================

function formatFileSize(bytes: number): string {
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  if (bytes >= 1_000) return `${(bytes / 1_000).toFixed(0)} KB`;
  return `${bytes} B`;
}

function formatTime(ts: number): string {
  const diff = Date.now() - ts;
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "刚刚";
  if (mins < 60) return `${mins} 分钟前`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} 天前`;
  return new Date(ts).toLocaleDateString("zh-CN");
}

function countFiles(items: KnowledgeFile[]): { folders: number; files: number } {
  let folders = 0;
  let files = 0;
  for (const item of items) {
    if (item.type === "folder") {
      folders++;
      if (item.children) {
        const sub = countFiles(item.children);
        folders += sub.folders;
        files += sub.files;
      }
    } else {
      files++;
    }
  }
  return { folders, files };
}

function flatSearch(items: KnowledgeFile[], query: string): KnowledgeFile[] {
  const results: KnowledgeFile[] = [];
  for (const item of items) {
    if (item.name.toLowerCase().includes(query) || item.tags?.some((t) => t.toLowerCase().includes(query))) {
      results.push(item);
    }
    if (item.children) {
      results.push(...flatSearch(item.children, query));
    }
  }
  return results;
}

function sortFiles(items: KnowledgeFile[], sortBy: SortBy): KnowledgeFile[] {
  const folders = items.filter((i) => i.type === "folder");
  const files = items.filter((i) => i.type !== "folder");

  const compareFn = (a: KnowledgeFile, b: KnowledgeFile) => {
    if (sortBy === "name") return a.name.localeCompare(b.name, "zh-CN");
    if (sortBy === "updated") return b.updatedAt - a.updatedAt;
    if (sortBy === "size") return (b.size || 0) - (a.size || 0);
    return 0;
  };

  return [...folders.sort(compareFn), ...files.sort(compareFn)];
}

// =============================================================================
// Sidebar tree node
// =============================================================================

function SidebarTreeNode({
  item,
  depth,
  activeFolderId,
  onSelect,
}: {
  item: KnowledgeFile;
  depth: number;
  activeFolderId: string | null;
  onSelect: (id: string, path: KnowledgeFile[]) => void;
}) {
  const [expanded, setExpanded] = useState(depth === 0);
  const isFolder = item.type === "folder";
  const isActive = activeFolderId === item.id;

  if (!isFolder) return null;

  const handleClick = () => {
    setExpanded(!expanded);
    onSelect(item.id, []);
  };

  return (
    <div>
      <button
        type="button"
        className={cn(
          "flex items-center gap-1.5 w-full rounded-md px-2 py-1.5 text-xs transition-colors",
          isActive
            ? "bg-primary/10 text-primary font-medium"
            : "text-muted-foreground hover:bg-foreground/[0.04] hover:text-foreground",
        )}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
        onClick={handleClick}
      >
        {item.children && item.children.some((c) => c.type === "folder") ? (
          expanded ? (
            <ChevronDown className="size-3 shrink-0" />
          ) : (
            <ChevronRight className="size-3 shrink-0" />
          )
        ) : (
          <span className="size-3 shrink-0" />
        )}
        <FolderIcon open={expanded} className="size-4 shrink-0" />
        <span className="flex-1 text-left truncate">{item.name}</span>
        {item.children && (
          <span className="text-[10px] text-muted-foreground">{item.children.filter((c) => c.type !== "folder").length}</span>
        )}
      </button>
      {expanded &&
        item.children
          ?.filter((c) => c.type === "folder")
          .map((child) => (
            <SidebarTreeNode
              key={child.id}
              item={child}
              depth={depth + 1}
              activeFolderId={activeFolderId}
              onSelect={onSelect}
            />
          ))}
    </div>
  );
}

// =============================================================================
// File row (list view)
// =============================================================================

function FileRow({ file, onClick }: { file: KnowledgeFile; onClick: () => void }) {
  const isFolder = file.type === "folder";
  const childCount = isFolder ? file.children?.length || 0 : 0;

  return (
    <div
      className="flex items-center gap-3 px-4 py-2.5 hover:bg-foreground/[0.03] transition-colors cursor-pointer group"
      onClick={onClick}
      onKeyDown={(e) => e.key === "Enter" && onClick()}
      role="button"
      tabIndex={0}
    >
      {isFolder ? (
        <FolderIcon className="size-5 shrink-0" />
      ) : (
        <FileIcon type={file.type} className="size-5 shrink-0" />
      )}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm truncate">{file.name}</span>
          {file.starred && <Star className="size-3 text-yellow-500 fill-yellow-500 shrink-0" />}
        </div>
        {file.tags && file.tags.length > 0 && (
          <div className="flex gap-1 mt-0.5">
            {file.tags.map((tag) => (
              <span key={tag} className="text-[10px] bg-primary/10 text-primary px-1.5 py-0.5 rounded">
                {tag}
              </span>
            ))}
          </div>
        )}
      </div>
      <div className="flex items-center gap-4 text-[11px] text-muted-foreground shrink-0">
        {isFolder ? (
          <span className="w-16 text-right">{childCount} 项</span>
        ) : (
          <span className="w-16 text-right">{file.size ? formatFileSize(file.size) : "-"}</span>
        )}
        <span className="w-20 text-right">{formatTime(file.updatedAt)}</span>
        {file.updatedBy && <span className="w-20 text-right truncate">{file.updatedBy}</span>}
      </div>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            className="size-7 p-0 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
            onClick={(e) => e.stopPropagation()}
          >
            <MoreHorizontal className="size-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-40">
          <DropdownMenuItem className="text-xs gap-2">
            <Eye className="size-3.5" /> 预览
          </DropdownMenuItem>
          <DropdownMenuItem className="text-xs gap-2">
            <Download className="size-3.5" /> 下载
          </DropdownMenuItem>
          <DropdownMenuItem className="text-xs gap-2">
            <Pencil className="size-3.5" /> 重命名
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem className="text-xs gap-2 text-red-600 dark:text-red-400">
            <Trash2 className="size-3.5" /> 删除
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

// =============================================================================
// File card (grid view)
// =============================================================================

function FileCard({ file, onClick }: { file: KnowledgeFile; onClick: () => void }) {
  const isFolder = file.type === "folder";
  const childCount = isFolder ? file.children?.length || 0 : 0;

  return (
    <div
      className="rounded-lg border bg-card p-3 hover:shadow-sm transition-shadow cursor-pointer group"
      onClick={onClick}
      onKeyDown={(e) => e.key === "Enter" && onClick()}
      role="button"
      tabIndex={0}
    >
      <div className="flex items-start justify-between mb-3">
        {isFolder ? (
          <FolderIcon className="size-8" />
        ) : (
          <FileIcon type={file.type} className="size-8" />
        )}
        <div className="flex items-center gap-1">
          {file.starred && <Star className="size-3 text-yellow-500 fill-yellow-500" />}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="size-6 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
                onClick={(e) => e.stopPropagation()}
              >
                <MoreHorizontal className="size-3.5" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-40">
              <DropdownMenuItem className="text-xs gap-2">
                <Eye className="size-3.5" /> 预览
              </DropdownMenuItem>
              <DropdownMenuItem className="text-xs gap-2">
                <Download className="size-3.5" /> 下载
              </DropdownMenuItem>
              <DropdownMenuItem className="text-xs gap-2">
                <Pencil className="size-3.5" /> 重命名
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem className="text-xs gap-2 text-red-600 dark:text-red-400">
                <Trash2 className="size-3.5" /> 删除
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
      <p className="text-xs font-medium truncate mb-1">{file.name}</p>
      <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
        {isFolder ? <span>{childCount} 项</span> : <span>{file.size ? formatFileSize(file.size) : ""}</span>}
        <span>{formatTime(file.updatedAt)}</span>
      </div>
      {file.tags && file.tags.length > 0 && (
        <div className="flex gap-1 mt-1.5 flex-wrap">
          {file.tags.map((tag) => (
            <span key={tag} className="text-[10px] bg-primary/10 text-primary px-1.5 py-0.5 rounded">
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Breadcrumb
// =============================================================================

function Breadcrumb({
  path,
  onNavigate,
}: {
  path: KnowledgeFile[];
  onNavigate: (index: number) => void;
}) {
  return (
    <div className="flex items-center gap-1 text-xs text-muted-foreground">
      <button
        type="button"
        className={cn(
          "hover:text-foreground transition-colors",
          path.length === 0 && "text-foreground font-medium",
        )}
        onClick={() => onNavigate(-1)}
      >
        知识库
      </button>
      {path.map((item, idx) => (
        <span key={item.id} className="flex items-center gap-1">
          <ChevronRight className="size-3" />
          <button
            type="button"
            className={cn(
              "hover:text-foreground transition-colors",
              idx === path.length - 1 && "text-foreground font-medium",
            )}
            onClick={() => onNavigate(idx)}
          >
            {item.name}
          </button>
        </span>
      ))}
    </div>
  );
}

// =============================================================================
// Main Knowledge Page
// =============================================================================

export default function KnowledgePage() {
  const [search, setSearch] = useState("");
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  const [sortBy, setSortBy] = useState<SortBy>("updated");
  const [folderPath, setFolderPath] = useState<KnowledgeFile[]>([]);
  const [activeFolderId, setActiveFolderId] = useState<string | null>(null);

  const q = search.trim().toLowerCase();

  // Current folder contents
  const currentItems = useMemo(() => {
    if (q) {
      return flatSearch(MOCK_KNOWLEDGE, q).filter((f) => f.type !== "folder");
    }
    if (folderPath.length === 0) {
      return MOCK_KNOWLEDGE;
    }
    const current = folderPath[folderPath.length - 1];
    return current.children || [];
  }, [folderPath, q]);

  const sorted = useMemo(() => sortFiles(currentItems, sortBy), [currentItems, sortBy]);

  const stats = useMemo(() => countFiles(MOCK_KNOWLEDGE), []);

  const handleFileClick = (file: KnowledgeFile) => {
    if (file.type === "folder") {
      setFolderPath([...folderPath, file]);
      setActiveFolderId(file.id);
      setSearch("");
    }
  };

  const handleBreadcrumbNav = (index: number) => {
    if (index === -1) {
      setFolderPath([]);
      setActiveFolderId(null);
    } else {
      setFolderPath(folderPath.slice(0, index + 1));
      setActiveFolderId(folderPath[index].id);
    }
    setSearch("");
  };

  const handleSidebarSelect = (id: string) => {
    // Find the folder path to this id
    const findPath = (items: KnowledgeFile[], target: string, path: KnowledgeFile[]): KnowledgeFile[] | null => {
      for (const item of items) {
        if (item.id === target) return [...path, item];
        if (item.children) {
          const found = findPath(item.children, target, [...path, item]);
          if (found) return found;
        }
      }
      return null;
    };

    const path = findPath(MOCK_KNOWLEDGE, id, []);
    if (path) {
      setFolderPath(path);
      setActiveFolderId(id);
      setSearch("");
    }
  };

  return (
    <div className="flex h-full w-full">
      {/* Left: folder tree sidebar */}
      <div className="w-56 border-r flex flex-col">
        <div className="px-4 py-3 border-b">
          <h2 className="text-sm font-semibold flex items-center gap-2">
            <BookOpen className="size-4 text-primary" />
            企业知识库
          </h2>
          <p className="text-[11px] text-muted-foreground mt-0.5">
            {stats.folders} 个文件夹，{stats.files} 个文件
          </p>
        </div>
        <ScrollArea className="flex-1">
          <div className="p-2 space-y-0.5">
            {MOCK_KNOWLEDGE.map((item) => (
              <SidebarTreeNode
                key={item.id}
                item={item}
                depth={0}
                activeFolderId={activeFolderId}
                onSelect={handleSidebarSelect}
              />
            ))}
          </div>

          {/* Storage usage */}
          <div className="px-4 py-3 border-t">
            <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground mb-2">
              <HardDrive className="size-3" />
              存储用量
            </div>
            <div className="h-1.5 bg-muted rounded-full overflow-hidden mb-1">
              <div className="h-full bg-primary/60 rounded-full" style={{ width: "34%" }} />
            </div>
            <span className="text-[10px] text-muted-foreground">已用 68.4 MB / 200 MB</span>
          </div>
        </ScrollArea>
      </div>

      {/* Right: file browser */}
      <div className="flex-1 flex flex-col">
        {/* Toolbar */}
        <div className="px-4 py-3 border-b flex items-center gap-3">
          <Breadcrumb path={folderPath} onNavigate={handleBreadcrumbNav} />
          <div className="flex-1" />
          <div className="relative max-w-xs">
            <Search className="absolute left-2.5 top-2 size-4 text-muted-foreground" />
            <Input
              placeholder="搜索文件..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-8 h-8 text-xs w-48"
            />
          </div>
          {/* Sort */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="sm" className="h-8 text-xs gap-1">
                <Clock className="size-3" />
                {sortBy === "updated" ? "最近更新" : sortBy === "name" ? "名称" : "大小"}
                <ChevronDown className="size-3" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-32">
              <DropdownMenuItem className="text-xs" onClick={() => setSortBy("updated")}>
                最近更新
              </DropdownMenuItem>
              <DropdownMenuItem className="text-xs" onClick={() => setSortBy("name")}>
                名称
              </DropdownMenuItem>
              <DropdownMenuItem className="text-xs" onClick={() => setSortBy("size")}>
                大小
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          {/* View mode toggle */}
          <div className="flex border rounded-md">
            <Button
              variant="ghost"
              size="sm"
              className={cn("h-8 w-8 p-0 rounded-r-none", viewMode === "list" && "bg-foreground/[0.06]")}
              onClick={() => setViewMode("list")}
            >
              <List className="size-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className={cn("h-8 w-8 p-0 rounded-l-none", viewMode === "grid" && "bg-foreground/[0.06]")}
              onClick={() => setViewMode("grid")}
            >
              <Grid3X3 className="size-3.5" />
            </Button>
          </div>
          {/* Actions */}
          <Button size="sm" className="h-8 text-xs gap-1">
            <Upload className="size-3" />
            上传
          </Button>
          <Button variant="outline" size="sm" className="h-8 text-xs gap-1">
            <Plus className="size-3" />
            新建文件夹
          </Button>
        </div>

        {/* File list / grid */}
        <ScrollArea className="flex-1">
          {viewMode === "list" ? (
            <div>
              {/* Column headers */}
              <div className="flex items-center gap-3 px-4 py-2 border-b text-[10px] text-muted-foreground font-medium uppercase tracking-wider">
                <span className="flex-1">名称</span>
                <span className="w-16 text-right">大小</span>
                <span className="w-20 text-right">更新时间</span>
                <span className="w-20 text-right">更新者</span>
                <span className="w-7" />
              </div>
              {sorted.map((file) => (
                <FileRow key={file.id} file={file} onClick={() => handleFileClick(file)} />
              ))}
            </div>
          ) : (
            <div className="p-4 grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-3">
              {sorted.map((file) => (
                <FileCard key={file.id} file={file} onClick={() => handleFileClick(file)} />
              ))}
            </div>
          )}

          {sorted.length === 0 && (
            <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
              <File className="size-10 mb-3 opacity-30" />
              <p className="text-sm">{q ? "未找到匹配的文件" : "此文件夹为空"}</p>
            </div>
          )}
        </ScrollArea>
      </div>
    </div>
  );
}

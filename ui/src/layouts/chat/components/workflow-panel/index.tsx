import { Button } from "@/components/ui/button";
import {
	FileCode2,
	FileUser,
	Newspaper,
	Plus
} from "lucide-react";
import Header from "./header";

export default function WorkflowPanel() {
	return (
		<div className="flex flex-col h-full">
			<Header />
			<div className="p-2">
				<Button className="w-full" variant="outline">
					<Plus className="size-4" />
					新增
				</Button>
			</div>
			<div className="flex flex-col items-center space-y-1 text-sm px-2">
				<div className="flex items-center space-x-2 w-full p-2 bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground cursor-pointer rounded">
					<FileUser className="size-4" />
					<span>简历筛选</span>
				</div>
				<div className="flex items-center space-x-2 w-full p-2 hover:bg-primary hover:text-primary-foreground cursor-pointer rounded">
					<Newspaper className="size-4" />
					<span>文章创作</span>
				</div>
			</div>
		</div>
	);
}

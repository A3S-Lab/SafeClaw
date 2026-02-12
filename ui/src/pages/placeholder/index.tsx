import { Construction } from "lucide-react";

export default function PlaceholderPage() {
	return (
		<div className="flex h-full w-full items-center justify-center">
			<div className="flex flex-col items-center gap-3 text-muted-foreground">
				<Construction className="size-10" />
				<span className="text-sm font-medium">即将推出</span>
			</div>
		</div>
	);
}

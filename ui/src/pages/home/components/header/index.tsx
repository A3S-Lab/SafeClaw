import { Sheet, SheetContent, SheetTrigger } from "@/components/ui/sheet";
import { Ellipsis } from "lucide-react";
import "./index.css";

export default function Header() {
	return (
		<div className="flex items-center justify-between border-b h-12 py-2 px-3 space-x-1 bg-secondary">
			<div className="text-mono flex-1">楚留香</div>
			<div className="flex items-center space-x-1">
				<Sheet>
					<SheetTrigger>
						<Ellipsis className="size-5 text-black/80 hover:text-black" />
					</SheetTrigger>
					<SheetContent className="container"></SheetContent>
				</Sheet>
			</div>
		</div>
	);
}

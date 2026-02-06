import { Input } from "@/components/ui/input";
import { useDebounceEffect } from "ahooks";
import { CircleX, Search } from "lucide-react";
import { useState } from "react";

export default function Header() {
	const [value, setValue] = useState("");
	useDebounceEffect(
		() => {
			console.log(value);
		},
		[value],
		{
			wait: 1000,
		},
	);
	return (
		<div className="flex h-12 border-b p-2 space-x-1">
			<div className="relative flex-1">
				<Search className="absolute size-4 left-2 top-2 text-muted-foreground/50" />
				<Input
					className="flex h-8 w-full rounded-md border border-input bg-background px-3 py-2 text-xs placeholder:text-muted-foreground/50 focus:border-primary focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 ps-8 pe-8"
					placeholder="搜索"
					value={value}
					onChange={(e) => setValue(e.target.value)}
				/>
				{value && (
					<CircleX
						className="absolute size-4 right-2 top-2 text-muted-foreground/50"
						onClick={() => setValue("")}
					/>
				)}
			</div>
		</div>
	);
}

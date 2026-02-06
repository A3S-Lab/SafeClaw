import { ScrollArea } from "@/components/ui/scroll-area";
import chatModel from "@/models/chat.model";
import { useSnapshot } from "valtio";
import Header from "./header";
import Item, { ItemProps } from "./item";

const items: Partial<ItemProps>[] = [
	{
		name: "知识库",
		organization: "",
		avatar: "/icons/file-assistant.png",
	},
];

export default function MarketPanel() {
	const { activeMarket } = useSnapshot(chatModel.state);
	return (
		<div className="flex flex-col h-full">
			<Header />
			<ScrollArea className="flex flex-col flex-1">
				{/* {items.map((item, index) => (
					<Item
						key={`${item.name}-${index}`}
						{...item}
						name={item.name!}
						isActive={activeMarket === index.toString()}
						onClick={() => chatModel.setActiveMarket(index.toString())}
					/>
				))} */}
			</ScrollArea>
		</div>
	);
}

import { ScrollArea } from "@/components/ui/scroll-area";
import chatModel from "@/models/chat.model";
import { useSnapshot } from "valtio";
import Header from "./header";
import Item, { ItemProps } from "./item";
import { RadioTower } from "lucide-react";

const items: Partial<ItemProps>[] = [
	{
		name: "知识库",
		message: "[视频]",
		avatar: "/icons/file-assistant.png",
	},
	{
		name: "情报官",
		message: "最新人工智能发展报告",
		avatar: () => (
			<div className="relative flex justify-center items-center rounded text-white size-10 min-w-10 bg-blue-500">
				<RadioTower />
				<div className="absolute -top-1 -right-1">
					<span className="relative flex h-2 w-2">
						<span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-rose-500 opacity-75" />
						<span className="relative inline-flex rounded-full h-2 w-2 bg-rose-500" />
					</span>
				</div>
			</div>
		),
		date: "2024-11-01 16:32:00",
	},
	{
		name: "楚留香",
		message: "楚留香: 这是为您定制的研发计划",
		avatar: {
			sex: "man",
			faceColor: "#F9C9B6",
			earSize: "big",
			eyeStyle: "circle",
			noseStyle: "long",
			mouthStyle: "peace",
			shirtStyle: "polo",
			glassesStyle: "round",
			hairColor: "#506AF4",
			hairStyle: "thick",
			hatStyle: "none",
			hatColor: "#F48150",
			shirtColor: "#F4D150",
			bgColor: "linear-gradient(45deg, #176fff 0%, #68ffef 100%)",
		},
		date: "2024-11-01 10:08:00",
	},
	{
		name: "法务部",
		message: "重磅:《2025最新法律法规解读》",
		avatar: {
			sex: "man",
			faceColor: "#F9C9B6",
			earSize: "big",
			eyeStyle: "oval",
			noseStyle: "round",
			mouthStyle: "laugh",
			shirtStyle: "hoody",
			glassesStyle: "none",
			hairColor: "#000",
			hairStyle: "thick",
			hatStyle: "none",
			hatColor: "#FC909F",
			shirtColor: "#77311D",
			bgColor: "#FC909F",
		},
		count: 1,
		date: "2024-11-01 10:22:00",
	},
	{
		name: "运营中心",
		message: "拍了拍你",
		isMute: true,
		avatar: {
			sex: "woman",
			faceColor: "#F9C9B6",
			earSize: "small",
			eyeStyle: "circle",
			noseStyle: "short",
			mouthStyle: "laugh",
			shirtStyle: "short",
			glassesStyle: "none",
			hairColor: "#000",
			hairStyle: "womanLong",
			hatStyle: "none",
			hatColor: "#77311D",
			shirtColor: "#6BD9E9",
			bgColor: "linear-gradient(90deg, #36cd1c 0%, #68deff 100%)",
		},
		date: "2024-11-01 11:58:00",
	},
];

export default function ChatPanel() {
	const { activeRoom } = useSnapshot(chatModel.state);
	return (
		<div className="flex flex-col h-full">
			<Header />
			<ScrollArea className="flex flex-col flex-1">
				{items.map((item, index) => (
					<Item
						key={`${item.name}-${index}`}
						{...item}
						name={item.name!}
						isActive={activeRoom === index.toString()}
						onClick={() => chatModel.setActiveRoom(index.toString())}
					/>
				))}
			</ScrollArea>
		</div>
	);
}

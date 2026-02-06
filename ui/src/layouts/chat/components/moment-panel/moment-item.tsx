import {
	MediaPlayer,
	MediaPlayerInstance,
	MediaProvider,
} from "@vidstack/react";
import dayjs from "dayjs";
import { Heart, Video } from "lucide-react";
import Avatar, { NiceAvatarProps, genConfig } from "react-nice-avatar";
import { EffectCards } from "swiper/modules";
import { Swiper, SwiperSlide } from "swiper/react";

import "@vidstack/react/player/styles/base.css";
import "swiper/css";
import "swiper/css/effect-cards";
import "./moment-item.css";
import { useRef } from "react";

interface UserInfo {
	id: string;
	name: string;
	avatar?: string | NiceAvatarProps;
}

export interface MomentItemProps {
	id: string;
	user: UserInfo;
	content?: string;
	organization?: string;
	type:
		| "message"
		| "blog"
		| "image"
		| "images"
		| "video"
		| "videos"
		| "link"
		| "movie"
		| "music"
		| "game"
		| "book"
		| "travel";
	data?: any;
	address?: string;
	date: string;
	likes?: UserInfo[];
}

const VideoPlayer = ({ id, url }: { id: string; url: string }) => {
	const player = useRef<MediaPlayerInstance>(null);
	return (
		<MediaPlayer
			ref={player}
			className="w-full h-full rounded-2xl"
			playsInline
			src={url}
			onClick={() => {
				player?.current?.play();
			}}
		>
			<MediaProvider />
		</MediaPlayer>
	);
};

export default function MomentItem({
	id,
	user,
	content,
	organization,
	type,
	data,
	address,
	date,
	likes,
}: MomentItemProps) {
	const renderMovie = (data: any) => {
		return (
			<div className="relative mt-2">
				<img className="rounded shadow-lg" src={data.cover} alt="" />

				<div className="absolute bottom-0 z-1 right-0 w-full rounded-b flex flex-col p-4 bg-gradient-to-b from-transparent via-transparent to-black">
					<span className="text-lg font-bold">{data.name}</span>
					<div className="flex items-center justify-between">
						<div className="bg-green-600 text-white py-1 px-2 rounded">
							{data.rating}
						</div>
						<div className="text-white text-sm font-bold">{data.year}</div>
					</div>
				</div>
			</div>
		);
	};

	const renderLink = (data: any) => {
		return (
			<div className="flex items-center space-x-2 mt-2 p-2 bg-gray-100 rounded-md max-w-[300px]">
				<img className="size-10" alt="logo" src={data.logo} />
				<span className="text-sm">{data.title}</span>
			</div>
		);
	};

	const renderVideos = (data: any) => {
		return (
			<div className="mt-2 p-2">
				<Swiper
					className="w-[400px] max-h-[400px] bg-transparent"
					effect={"cards"}
					grabCursor={true}
					modules={[EffectCards]}
				>
					{data?.map((item: any) => {
						return (
							<SwiperSlide key={item.id}>
								<VideoPlayer {...item} />
							</SwiperSlide>
						);
					})}
				</Swiper>
			</div>
		);
	};

	return (
		<div
			className="flex space-x-2 pr-6"
			onMouseDown={(e) => {
				e.stopPropagation();
			}}
		>
			<div className="flex">
				{typeof user.avatar === "string" ? (
					<img
						className="size-10 min-w-10 rounded"
						alt="avatar"
						src={user.avatar}
					/>
				) : (
					<Avatar
						className="size-10 min-w-10"
						shape="rounded"
						{...genConfig(user.avatar)}
					/>
				)}
			</div>
			<div className="flex flex-col pb-4 border-b border-muted w-full">
				<div className="flex items-center space-x-2 text-moment-primary font-mono text-sm whitespace-nowrap text-ellipsis">
					<span>{user.name}</span>
					{organization && (
						<span className="text-market-primary">@{organization}</span>
					)}
				</div>
				{content && (
					<div className="flex flex-col space-y-1 mt-2 text-foreground text-sm">
						{content.split("\n").map((line: string, index: number) => (
							<p key={index}>{line}</p>
						))}
					</div>
				)}
				{"movie" === type && renderMovie(data)}
				{"link" === type && renderLink(data)}
				{"videos" === type && renderVideos(data)}
				{address && (
					<div className="flex justify-between items-center">
						<div className="mt-2 text-xs text-muted-foreground/50">
							{address}
						</div>
					</div>
				)}
				<div className="flex justify-between items-center mt-2">
					<div className="mt-2 text-xs text-muted-foreground/50">
						{dayjs(date).fromNow()}
					</div>
					<div className="flex items-center space-x-1 bg-gray-100 py-2 px-3 rounded-md">
						<span className="size-[4px] rounded-full bg-moment-primary" />
						<span className="size-[4px] rounded-full bg-moment-primary" />
					</div>
				</div>
				{likes && likes?.length > 0 && (
					<div className="flex items-center space-x-2 mt-2 py-2 px-3 rounded bg-gray-100 text-moment-primary text-sm">
						<Heart className="size-4" />
						{likes?.map((like, index) => (
							<div key={index}>{like.name}</div>
						))}
					</div>
				)}
			</div>
		</div>
	);
}

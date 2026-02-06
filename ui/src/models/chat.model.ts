import { proxy } from "valtio";

const state = proxy({
	momentPanelOpen: false,
	activeKey: "chat",
	activeRoom: "2",
	activeMarket: "0",
});

const actions = {
	setActiveKey: (key: string) => {
		state.activeKey = key;
	},
	setActiveRoom: (room: string) => {
		state.activeRoom = room;
	},
	setActiveMarket: (room: string) => {
		state.activeMarket = room;
	},
	setMomentPanelOpen: (open: boolean) => {
		state.momentPanelOpen = open;
	},
};

export default {
	state,
	...actions,
};

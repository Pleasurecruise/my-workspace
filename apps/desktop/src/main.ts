import { mount } from "svelte";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "@my-workspace/ui/styles";

const target = document.getElementById("app");

if (!target) {
	throw new Error("Application mount target was not found");
}

if (getCurrentWindow().label === "island") {
	document.documentElement.classList.add("dark");
	void import("./IslandApp.svelte").then(({ default: IslandApp }) => mount(IslandApp, { target }));
} else {
	void import("./App.svelte").then(({ default: App }) => mount(App, { target }));
}

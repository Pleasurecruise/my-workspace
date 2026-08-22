import { mount } from "svelte";
import App from "./App.svelte";
import "@my-workspace/ui/styles";

const target = document.getElementById("app");

if (!target) {
  throw new Error("Application mount target was not found");
}

export default mount(App, { target });

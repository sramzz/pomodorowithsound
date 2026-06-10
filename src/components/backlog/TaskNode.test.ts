import { afterEach, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createTestingPinia } from "@pinia/testing";
import TaskNode from "./TaskNode.vue";
import { useProjectStore } from "../../stores/projectStore";

const task = {
  id: "t1", title: "Task", description: "Notes", deadline: null,
  priority: 2, status: "open" as const, microtasks: [],
};

afterEach(() => vi.restoreAllMocks());

it("exposes edit, archive, and delete actions", async () => {
  vi.spyOn(window, "prompt").mockReturnValue("Renamed task");
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const wrapper = mount(TaskNode, {
    props: { task },
    global: { plugins: [createTestingPinia({ createSpy: vi.fn })] },
  });
  const store = useProjectStore();

  await wrapper.get('[aria-label="Edit task"]').trigger("click");
  await wrapper.get('[aria-label="Archive task"]').trigger("click");
  await wrapper.get('[aria-label="Delete task"]').trigger("click");

  expect(store.updateTask).toHaveBeenCalledWith("t1", "Renamed task", "Notes", null, 2);
  expect(store.archiveTask).toHaveBeenCalledWith("t1");
  expect(store.deleteTask).toHaveBeenCalledWith("t1");
});

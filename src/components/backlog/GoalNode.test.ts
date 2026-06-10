import { afterEach, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createTestingPinia } from "@pinia/testing";
import GoalNode from "./GoalNode.vue";
import { useProjectStore } from "../../stores/projectStore";

const goal = {
  id: "g1", title: "Goal", description: null, deadline: null,
  priority: 0, status: "open" as const,
  tasks: [
    { id: "t1", title: "A", description: null, deadline: null, priority: 0, status: "open" as const, microtasks: [] },
    { id: "t2", title: "B", description: null, deadline: null, priority: 0, status: "open" as const, microtasks: [] },
  ],
};

afterEach(() => vi.restoreAllMocks());

it("sends the full ordered task id list after a drop", async () => {
  const wrapper = mount(GoalNode, {
    props: { goal },
    global: { plugins: [createTestingPinia({ createSpy: vi.fn })] },
  });
  const store = useProjectStore();

  // simulate vuedraggable's post-drop state: local list already reordered
  wrapper.vm.localTasks.reverse();
  await wrapper.vm.onTaskDrop();

  expect(store.reorderTasks).toHaveBeenCalledWith("g1", ["t2", "t1"]);
});

it("exposes edit, archive, and delete actions", async () => {
  vi.spyOn(window, "prompt").mockReturnValue("Renamed goal");
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const wrapper = mount(GoalNode, {
    props: { goal },
    global: { plugins: [createTestingPinia({ createSpy: vi.fn })] },
  });
  const store = useProjectStore();

  await wrapper.get('[aria-label="Edit goal"]').trigger("click");
  await wrapper.get('[aria-label="Archive goal"]').trigger("click");
  await wrapper.get('[aria-label="Delete goal"]').trigger("click");

  expect(store.updateGoal).toHaveBeenCalledWith("g1", "Renamed goal", null, null, 0);
  expect(store.archiveGoal).toHaveBeenCalledWith("g1");
  expect(store.deleteGoal).toHaveBeenCalledWith("g1");
});

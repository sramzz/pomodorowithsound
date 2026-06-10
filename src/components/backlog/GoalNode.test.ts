import { expect, it, vi } from "vitest";
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

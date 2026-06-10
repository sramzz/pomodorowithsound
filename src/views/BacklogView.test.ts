import { afterEach, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createTestingPinia } from "@pinia/testing";
import BacklogView from "./BacklogView.vue";
import { useProjectStore } from "../stores/projectStore";

afterEach(() => vi.restoreAllMocks());

it("confirms before deleting a project", async () => {
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const pinia = createTestingPinia({
    createSpy: vi.fn,
    initialState: {
      project: {
        projects: [{
          id: "p1", name: "Project", description: null, status: "open",
          isArchived: false, completedAt: null, createdAt: "now", updatedAt: "now",
          totalMicrotasks: 0, completedMicrotasks: 0,
        }],
      },
    },
  });
  const wrapper = mount(BacklogView, {
    global: {
      plugins: [pinia],
      stubs: { draggable: true, GoalNode: true, InlineCreate: true },
    },
  });
  const store = useProjectStore();

  await wrapper.get('[aria-label="Delete Project"]').trigger("click");

  expect(window.confirm).toHaveBeenCalled();
  expect(store.deleteProject).toHaveBeenCalledWith("p1");
});

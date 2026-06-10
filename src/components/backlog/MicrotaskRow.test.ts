import { afterEach, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createTestingPinia } from "@pinia/testing";
import MicrotaskRow from "./MicrotaskRow.vue";
import { useProjectStore } from "../../stores/projectStore";

const microtask = {
  id: "m1", title: "Microtask", estimatedMinutes: 40, pomodoroCount: 2,
  pomodoroTypeId: null, deadline: null, priority: 1, status: "open" as const,
};

afterEach(() => vi.restoreAllMocks());

it("exposes edit, archive, and delete actions", async () => {
  vi.spyOn(window, "prompt")
    .mockReturnValueOnce("Renamed microtask")
    .mockReturnValueOnce("50")
    .mockReturnValueOnce("3");
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const wrapper = mount(MicrotaskRow, {
    props: { microtask },
    global: { plugins: [createTestingPinia({ createSpy: vi.fn })] },
  });
  const store = useProjectStore();

  await wrapper.get('[aria-label="Edit microtask"]').trigger("click");
  await wrapper.get('[aria-label="Archive microtask"]').trigger("click");
  await wrapper.get('[aria-label="Delete microtask"]').trigger("click");

  expect(store.updateMicrotask).toHaveBeenCalledWith(
    "m1", "Renamed microtask", 50, 3, null, null, 1,
  );
  expect(store.archiveMicrotask).toHaveBeenCalledWith("m1");
  expect(store.deleteMicrotask).toHaveBeenCalledWith("m1");
});

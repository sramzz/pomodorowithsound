import { afterEach, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createTestingPinia } from "@pinia/testing";
import PomodoroTypesSection from "./PomodoroTypesSection.vue";
import { usePomodoroTypeStore } from "../../stores/pomodoroTypeStore";

afterEach(() => vi.restoreAllMocks());

it("exposes preset editing", async () => {
  vi.spyOn(window, "prompt")
    .mockReturnValueOnce("Deep Work")
    .mockReturnValueOnce("50")
    .mockReturnValueOnce("10")
    .mockReturnValueOnce("")
    .mockReturnValueOnce("");
  const pinia = createTestingPinia({
    createSpy: vi.fn,
    initialState: {
      pomodoroType: {
        types: [{
          id: "pt1", name: "Deep", workMinutes: 45, restMinutes: 10,
          longBreakMinutes: 20, longBreakEvery: 4, isDefault: false,
        }],
      },
    },
  });
  const wrapper = mount(PomodoroTypesSection, { global: { plugins: [pinia] } });
  const store = usePomodoroTypeStore();

  await wrapper.get('[aria-label="Edit Deep"]').trigger("click");

  expect(store.updateType).toHaveBeenCalledWith("pt1", {
    name: "Deep Work",
    workMinutes: 50,
    restMinutes: 10,
    longBreakMinutes: null,
    longBreakEvery: null,
  });
});

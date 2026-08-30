/**
 * `src/components/measured-virtual-list.tsx` already wraps `@tanstack/react-virtual` with dynamic
 * measurement, stable item keys, and an imperative scroll/measure handle, and is proven across 15
 * existing call sites (trace waterfall, log tabs, prompt-hook cards, ...). Task 3.10 asks for a
 * `src/ui/virtual-list/` wrapper around the virtualization *dependency* — building a second one
 * from scratch would duplicate exactly the logic that component already gets right, so this
 * re-exports it under the new primitive namespace instead of forking it.
 */
export { MeasuredVirtualList as VirtualList, type MeasuredVirtualListHandle as VirtualListHandle, type MeasuredVirtualListProps as VirtualListProps } from "../../components/measured-virtual-list";

-- Unit tests for lua/hermes/queue.lua
-- Full coverage of all queue operations

describe("hermes.queue", function()
	local queue

	before_each(function()
		package.loaded["hermes.queue"] = nil
		queue = require("hermes.queue")
		-- Ensure queue starts empty
		queue.clear()
	end)

	after_each(function()
		queue.clear()
	end)

	describe("basic operations", function()
		it("push adds item to queue", function()
			local result = queue.push(function() end)
			-- Single assertion comparing both return value and side effect
			assert.same({
				result = result,
				size = queue.size(),
			}, {
				result = true,
				size = 1,
			})
		end)

		it("pop removes and returns item from front", function()
			local called = false
			queue.push(function() called = true end)
			local fn = queue.pop()
			local is_function = type(fn) == "function"
			fn()
			-- Single assertion comparing all related values
			assert.same({
				is_function = is_function,
				called = called,
				is_empty = queue.is_empty(),
			}, {
				is_function = true,
				called = true,
				is_empty = true,
			})
		end)

		it("pop returns nil when queue is empty", function()
			local fn = queue.pop()
			assert.is_nil(fn)
		end)

		it("is_empty returns true for empty queue", function()
			assert.is_true(queue.is_empty())
		end)

		it("is_empty returns false when queue has items", function()
			queue.push(function() end)
			assert.is_false(queue.is_empty())
		end)

		it("size returns 0 for empty queue", function()
			assert.equals(0, queue.size())
		end)

		it("size returns correct count for multiple items", function()
			queue.push(function() end)
			queue.push(function() end)
			queue.push(function() end)
			assert.equals(3, queue.size())
		end)
	end)

	describe("FIFO ordering", function()
		it("maintains FIFO order with push and pop", function()
			local order = {}
			queue.push(function() table.insert(order, 1) end)
			queue.push(function() table.insert(order, 2) end)
			queue.push(function() table.insert(order, 3) end)

			while not queue.is_empty() do
				local fn = queue.pop()
				fn()
			end

			assert.same({ 1, 2, 3 }, order)
		end)
	end)

	describe("clear operation", function()
		it("clears all items from queue", function()
			queue.push(function() end)
			queue.push(function() end)
			local cleared = queue.clear()
			-- Single assertion comparing all related values
			assert.same({
				cleared = 2,
				is_empty = queue.is_empty(),
				size = queue.size(),
			}, {
				cleared = cleared,
				is_empty = true,
				size = 0,
			})
		end)

		it("clear returns 0 for empty queue", function()
			local cleared = queue.clear()
			assert.equals(0, cleared)
		end)

		it("allows operations after clear", function()
			queue.push(function() end)
			queue.clear()
			queue.push(function() end)
			assert.equals(1, queue.size())
		end)
	end)

	describe("execute_all", function()
		it("executes all functions in order", function()
			local order = {}
			queue.push(function() table.insert(order, "a") end)
			queue.push(function() table.insert(order, "b") end)
			queue.push(function() table.insert(order, "c") end)

			local executed, err = queue.execute_all()

			-- Single assertion comparing all related values
			assert.same({
				executed = executed,
				err = err,
				order = order,
				is_empty = queue.is_empty(),
			}, {
				executed = 3,
				err = nil,
				order = { "a", "b", "c" },
				is_empty = true,
			})
		end)

		it("returns 0 for empty queue", function()
			local executed, err = queue.execute_all()
			-- Single assertion comparing both return values
			assert.same({ executed = executed, err = err }, { executed = 0, err = nil })
		end)

		it("stops on first error and returns error message", function()
			local order = {}
			queue.push(function() table.insert(order, 1) end)
			queue.push(function() error("test error") end)
			queue.push(function() table.insert(order, 3) end)

			local executed, err = queue.execute_all()

			-- Single assertion comparing all related values
			assert.same({
				executed = executed,
				has_error = err ~= nil,
				is_test_error = err and err:match("test error") ~= nil,
				order = order,
			}, {
				executed = 1,
				has_error = true,
				is_test_error = true,
				order = { 1 },
			})
		end)

		it("clears queue on error", function()
			queue.push(function() error("fail") end)
			queue.push(function() end)

			queue.execute_all()

			-- Single assertion comparing both queue state values
			assert.same({
				is_empty = queue.is_empty(),
				size = queue.size(),
			}, {
				is_empty = true,
				size = 0,
			})
		end)

		it("handles single item queue", function()
			local called = false
			queue.push(function() called = true end)

			local executed, err = queue.execute_all()

			-- Single assertion comparing all related values
			assert.same({
				executed = executed,
				err = err,
				called = called,
			}, {
				executed = 1,
				err = nil,
				called = true,
			})
		end)
	end)

	describe("edge cases", function()
		it("handles many push/pop operations", function()
			for _ = 1, 100 do
				queue.push(function() end)
			end
			local size_after_push = queue.size()

			for _ = 1, 100 do
				queue.pop()
			end
			local is_empty_after_pop = queue.is_empty()

			-- Single assertion comparing final state
			assert.same({
				size_after_push = size_after_push,
				is_empty_after_pop = is_empty_after_pop,
			}, {
				size_after_push = 100,
				is_empty_after_pop = true,
			})
		end)

		it("handles alternating push and pop", function()
			queue.push(function() end)
			queue.pop()
			queue.push(function() end)
			queue.pop()
			queue.push(function() end)

			assert.equals(1, queue.size())
		end)

		it("pop after many operations still works correctly", function()
			-- Simulate usage pattern: push many, pop many, push again
			for _ = 1, 50 do
				queue.push(function() end)
			end
			for _ = 1, 40 do
				queue.pop()
			end
			for _ = 1, 10 do
				queue.push(function() end)
			end

			local size_after_operations = queue.size()

			-- All remaining items should be executable
			local count = 0
			while not queue.is_empty() do
				local fn = queue.pop()
				if fn then
					count = count + 1
				end
			end

			-- Single assertion comparing both values
			assert.same({
				size_after_operations = size_after_operations,
				count = count,
			}, {
				size_after_operations = 20,
				count = 20,
			})
		end)
	end)
end)

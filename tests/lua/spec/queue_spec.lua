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
			assert.is_true(result)
			assert.equals(1, queue.size())
		end)

		it("pop removes and returns item from front", function()
			local called = false
			queue.push(function() called = true end)
			local fn = queue.pop()
			assert.is_function(fn)
			fn()
			assert.is_true(called)
			assert.is_true(queue.is_empty())
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
			assert.equals(2, cleared)
			assert.is_true(queue.is_empty())
			assert.equals(0, queue.size())
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

			assert.equals(3, executed)
			assert.is_nil(err)
			assert.same({ "a", "b", "c" }, order)
			assert.is_true(queue.is_empty())
		end)

		it("returns 0 for empty queue", function()
			local executed, err = queue.execute_all()
			assert.equals(0, executed)
			assert.is_nil(err)
		end)

		it("stops on first error and returns error message", function()
			local order = {}
			queue.push(function() table.insert(order, 1) end)
			queue.push(function() error("test error") end)
			queue.push(function() table.insert(order, 3) end)

			local executed, err = queue.execute_all()

			assert.equals(1, executed)
			assert.is_not_nil(err)
			assert.truthy(err:match("test error"))
			assert.same({ 1 }, order)
		end)

		it("clears queue on error", function()
			queue.push(function() error("fail") end)
			queue.push(function() end)

			queue.execute_all()

			assert.is_true(queue.is_empty())
			assert.equals(0, queue.size())
		end)

		it("handles single item queue", function()
			local called = false
			queue.push(function() called = true end)

			local executed, err = queue.execute_all()

			assert.equals(1, executed)
			assert.is_nil(err)
			assert.is_true(called)
		end)
	end)

	describe("edge cases", function()
		it("handles many push/pop operations", function()
			for i = 1, 100 do
				queue.push(function() end)
			end
			assert.equals(100, queue.size())

			for i = 1, 100 do
				queue.pop()
			end
			assert.is_true(queue.is_empty())
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
			for i = 1, 50 do
				queue.push(function() end)
			end
			for i = 1, 40 do
				queue.pop()
			end
			for i = 1, 10 do
				queue.push(function() end)
			end

			assert.equals(20, queue.size())

			-- All remaining items should be executable
			local count = 0
			while not queue.is_empty() do
				local fn = queue.pop()
				if fn then
					count = count + 1
				end
			end
			assert.equals(20, count)
		end)
	end)
end)

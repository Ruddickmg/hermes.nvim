-- luacov: disable
---@brief [[
--- Internal queue manager for storing deferred operations
--- Used for queuing API calls made before the binary is fully loaded
---
--- Design:
--- - O(1) push and pop operations using head/tail indices
--- - Executes all queued items in FIFO order when binary is ready
--- - Stops on first error and propagates it
--- - Clears queue on load failure
---@brief ]]

---@class Queue
-- luacov: enable

local M = {}

-- Private queue state using circular buffer approach
-- Using table with head/tail indices for O(1) operations
local _queue = {}
local _head = 1
local _tail = 1

-- luacov: disable
---Add a function to the back of the queue
---@param fn function The callback to queue
---@return boolean success Always returns true
---@private
-- luacov: enable
function M.push(fn)
	_queue[_tail] = fn
	_tail = _tail + 1
	return true
end

-- luacov: disable
---Remove and return a function from the front of the queue
---@return function|nil fn The queued function, or nil if empty
---@private
-- luacov: enable
function M.pop()
	if _head >= _tail then
		return nil
	end
	local fn = _queue[_head]
	_queue[_head] = nil -- Allow garbage collection
	_head = _head + 1
	return fn
end

-- luacov: disable
---Check if queue is empty
---@return boolean is_empty True if queue has no items
---@private
-- luacov: enable
function M.is_empty()
	return _head >= _tail
end

-- luacov: disable
---Get number of items in queue
---@return number size Number of queued items
---@private
-- luacov: enable
function M.size()
	return _tail - _head
end

-- luacov: disable
---Clear all items from queue
---@return number cleared_count Number of items cleared
---@private
-- luacov: enable
function M.clear()
	local count = M.size()
	_queue = {}
	_head = 1
	_tail = 1
	return count
end

-- luacov: disable
---Execute all queued functions in FIFO order
---Stops and propagates error on first failure
---Returns number of successfully executed functions
---@return number executed_count Number of functions executed
---@return nil|string error Error message if execution failed
---@private
-- luacov: enable
function M.execute_all()
	local executed = 0
	while not M.is_empty() do
		local fn = M.pop()
		if fn then
			local ok, err = pcall(fn)
			if not ok then
				-- Clear remaining queue on error
				M.clear()
				return executed, tostring(err)
			end
			executed = executed + 1
		end
	end
	return executed, nil
end

return M

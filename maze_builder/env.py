from typing import List
import numpy as np
from maze_builder.reward import compute_reward
from maze_builder.types import Room
from maze_builder.display import MapDisplay
import torch


class MazeBuilderEnv:
    def __init__(self, rooms: List[Room], map_x: int, map_y: int, action_radius: int,
                 num_envs: int, history_size: int, episode_length: int):
        for room in rooms:
            room.populate()

        self.rooms = rooms
        self.map_x = map_x
        self.map_y = map_y
        self.action_radius = action_radius
        self.num_envs = num_envs
        self.history_size = history_size
        self.episode_length = episode_length

        self.room_tensors = [torch.stack([torch.tensor(room.map).t(),
                                          torch.tensor(room.door_left).t(),
                                          torch.tensor(room.door_right).t(),
                                          torch.tensor(room.door_down).t(),
                                          torch.tensor(room.door_up).t()])
                            for room in rooms]
        self.cap_x = torch.tensor([map_x - room.width for room in rooms])
        self.cap_y = torch.tensor([map_y - room.height for room in rooms])
        self.cap = torch.stack([self.cap_x, self.cap_y], dim=1)
        assert torch.all(self.cap > 0)  # Ensure map is big enough for largest room in each direction
        self.action_width = 2 * action_radius + 1
        self.actions_per_room = self.action_width ** 2 - 1
        self.num_actions = len(rooms) * self.actions_per_room

        self.state = torch.zeros([num_envs, history_size, len(rooms), 2], dtype=torch.int64)
        self.action = torch.zeros([num_envs, history_size - 1], dtype=torch.int64)
        self.reward = torch.zeros([num_envs, history_size - 1], dtype=torch.float32)
        self.mask = torch.zeros([num_envs, history_size - 1], dtype=torch.bool)
        self.step_number = 1
        self.partial_reset(0, num_envs)

        self.map_display = None
        self.color_map = {0: (0xd0, 0x90, 0x90)}

    def partial_reset(self, start, end):
        rand_state = torch.randint(2 ** 30, [end - start, len(self.rooms), 2]) % self.cap.unsqueeze(0)
        self.state[start:end, 0, :, :] = rand_state
        self.mask[start:end, :] = False

    def staggered_reset(self):
        start = int(self.step_number / self.episode_length * self.num_envs)
        end = int((self.step_number + 1) / self.episode_length * self.num_envs)
        print(start, end)
        self.partial_reset(start, end)

    def step(self, action: torch.tensor):
        # Shift existing history back a time step, to make room for the new data
        self.state[:, 1:, :, :] = self.state[:, :-1, :, :]
        self.action[:, 1:] = self.action[:, :-1]
        self.reward[:, 1:] = self.reward[:, :-1]
        self.mask[:, 1:] = self.mask[:, :-1]

        # Decompose the raw action into its components (room_index and displacement):
        room_index = action // self.actions_per_room
        displacement_raw = action % self.actions_per_room
        displacement_coded = torch.where(displacement_raw >= (self.action_width ** 2 - 1) // 2,
                                   displacement_raw + 1,
                                   displacement_raw)
        displacement_x = displacement_coded % self.action_width - self.action_radius
        displacement_y = displacement_coded // self.action_width - self.action_radius
        displacement = torch.stack([displacement_x, displacement_y], dim=1)

        # Update the state
        old_state = self.state[torch.arange(self.num_envs), 0, room_index, :]
        new_state = torch.minimum(torch.clamp(old_state + displacement, min=0), self.cap[room_index, :])
        self.state[torch.arange(self.num_envs), 0, room_index, :] = new_state
        self.action[:, 0] = action
        self.reward[:, 0] = self._compute_reward(self.state[:, 0, :, :], action)
        self.mask[torch.arange(self.num_envs), 0] = True

        self.step_number = (self.step_number + 1) % self.episode_length

    def _compute_map(self, state: torch.tensor) -> torch.tensor:
        full_map = torch.zeros([self.num_envs, self.map_x, self.map_y], dtype=torch.int64)
        for k, room_tensor in enumerate(self.room_tensors):
            room_map = room_tensor[0, :, :]
            room_x = state[:, k, 0]
            room_y = state[:, k, 1]
            width = room_map.shape[0]
            height = room_map.shape[1]
            index_x = torch.arange(width).view(1, -1, 1) + room_x.view(-1, 1, 1)
            index_y = torch.arange(height).view(1, 1, -1) + room_y.view(-1, 1, 1)
            full_map[torch.arange(self.num_envs).view(-1, 1, 1), index_x, index_y] += room_map
        return full_map

    def _compute_intersection_cost(self, state: torch.tensor) -> int:
        full_map = self._compute_map(state)
        intersection_cost = torch.sum(torch.clamp(full_map - 1, min=0), dim=(1, 2))
        return intersection_cost

    def _compute_reward(self, state, action):
        intersection_cost = self._compute_intersection_cost(state)
        total_cost = intersection_cost
        return -total_cost

    def render(self, env_index=0):
        if self.map_display is None:
            self.map_display = MapDisplay(self.map_x, self.map_y)
        xs = self.state[env_index, 0, :, 0].tolist()
        ys = self.state[env_index, 0, :, 1].tolist()
        colors = [self.color_map[room.area] for room in self.rooms]
        self.map_display.display(self.rooms, xs, ys, colors)

    def close(self):
        pass

# import maze_builder.crateria
# num_envs = 3
# rooms = maze_builder.crateria.rooms
# action_radius = 2
# env = MazeBuilderEnv(rooms,
#                      map_x=40,
#                      map_y=20,
#                      action_radius=action_radius,
#                      num_envs=num_envs,
#                      history_size=5,
#                      episode_length=100)
# for i in range(200):
#     print(i)
#     env.render(2)
#     import time
#     time.sleep(0.1)
#     env.staggered_reset()
#     action = torch.randint(env.num_actions, [num_envs])
#     env.step(action)
#
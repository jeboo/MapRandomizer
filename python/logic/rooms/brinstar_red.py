from logic.areas import Area, SubArea
from maze_builder.types import Room, DoorIdentifier, Direction, DoorSubtype, Item

LEFT = Direction.LEFT
RIGHT = Direction.RIGHT
UP = Direction.UP
DOWN = Direction.DOWN
ELEVATOR = DoorSubtype.ELEVATOR

rooms = [
    Room(
        room_id=66,
        name='Red Tower',
        rom_address=0x7A253,
        map=[
            [1],
            [1],
            [1],
            [1],
            [1],
            [1],
            [1],
            [1],
            [1],
            [1],
        ],
        door_ids=[
            DoorIdentifier(1, LEFT, 0, 4, 0x1902A, 0x18F0A, 0),  # Noob Bridge
            DoorIdentifier(2, LEFT, 0, 6, 0x19036, 0x19066, 0),  # Red Brinstar Fireflea Room
            DoorIdentifier(3, LEFT, 0, 9, 0x1904E, 0x191FE, 0),  # Sloaters Refill
            DoorIdentifier(5, RIGHT, 0, 0, 0x1901E, 0x1907E, 0),  # Hellway
            DoorIdentifier(4, RIGHT, 0, 9, 0x19042, 0x190F6, 0),  # Bat Room
        ],
        node_tiles={
            1: [(0, 4), (0, 5), (0, 6)],  # top left door
            2: [(0, 6)],  # middle left door
            3: [(0, 9)],  # bottom left door
            4: [(0, 9)],  # bottom right door
            5: [(0, 0)],  # top right door
            6: [(0, 7)],  # junction below PBs
            7: [(0, 8), (0, 9)],  # ledge junction
            9: [(0, 1), (0, 2), (0, 3)],  # top junction
            10: [(0, 4)], # temporary blue junction (top left door)
        },
    ),
    Room(
        room_id=67,
        name='Red Brinstar Fireflea Room',
        rom_address=0x7A293,
        map=[
            [1, 1, 1, 1, 1, 1, 1, 1],
            [1, 1, 0, 0, 1, 1, 0, 0],
        ],
        door_ids=[
            DoorIdentifier(1, LEFT, 0, 0, 0x1905A, 0x19072, 0),  # X-Ray Scope Room
            DoorIdentifier(2, RIGHT, 7, 0, 0x19066, 0x19036, 0),  # Red Tower
        ],
        node_tiles={
            1: [(0, 0), (1, 0), (2, 0), (3, 0)],  # left door
            2: [(4, 0), (4, 1), (5, 0), (5, 1), (6, 0), (7, 0)],  # right door
            3: [(3, 0)],  # safe block in the middle of room
            4: [(0, 1), (1, 1)],  # small ledge below left door
        },
    ),
    Room(
        room_id=68,
        name='X-Ray Scope Room',
        rom_address=0x7A2CE,
        map=[[1, 1]],
        door_ids=[
            DoorIdentifier(1, RIGHT, 1, 0, 0x19072, 0x1905A, 0),  # Red Brinstar Fireflea Room
        ],
        items=[
            Item(0, 0, 0x78876),
        ],
        node_tiles={
            1: [(1, 0)],  # door
            2: [(0, 0)],  # xray
        },
    ),
    Room(
        room_id=69,
        name='Bat Room',
        rom_address=0x7A3DD,
        map=[[1, 1]],
        door_ids=[
            DoorIdentifier(1, LEFT, 0, 0, 0x190F6, 0x19042, 0),  # Red Tower
            DoorIdentifier(2, RIGHT, 1, 0, 0x19102, 0x1910E, 0),  # Below Spazer
        ],
        node_tiles={
            1: [(0, 0)],  # left door
            2: [(1, 0)],  # right door
        },
    ),
    Room(
        room_id=70,
        name='Below Spazer',
        rom_address=0x7A408,
        map=[
            [1, 1],
            [1, 1],
        ],
        door_ids=[
            DoorIdentifier(1, LEFT, 0, 1, 0x1910E, 0x19102, 0),  # Bat Room
            DoorIdentifier(3, RIGHT, 1, 0, 0x19126, 0x19132, 0),  # Spazer Room
            DoorIdentifier(2, RIGHT, 1, 1, 0x1911A, 0x1A36C, 0),  # West Tunnel
        ],
        node_tiles={
            1: [(0, 0), (0, 1)],  # left door
            2: [(1, 1)],  # bottom right door
            3: [(1, 0)],  # top right door
        },
    ),
    Room(
        room_id=71,
        name='Spazer Room',
        rom_address=0x7A447,
        map=[[1]],
        door_ids=[
            DoorIdentifier(1, LEFT, 0, 0, 0x19132, 0x19126, 0),  # Below Spazer
        ],
        items=[
            Item(0, 0, 0x7896E),
        ],
        node_tiles={
            1: [(0, 0)],  # door
            2: [(0, 0)],  # spazer
        },
    ),
    Room(
        room_id=74,
        name='Hellway',
        rom_address=0x7A2F7,
        map=[[1, 1, 1]],
        door_ids=[
            DoorIdentifier(1, LEFT, 0, 0, 0x1907E, 0x1901E, 0),  # Red Tower
            DoorIdentifier(2, RIGHT, 2, 0, 0x1908A, 0x190AE, 0),  # Caterpillar Room
        ],
        node_tiles={
            1: [(0, 0), (1, 0)],  # left door
            2: [(2, 0)],  # right door
        },
    ),
    Room(
        room_id=75,
        name='Caterpillar Room',
        rom_address=0x7A322,
        map=[
            [1, 0, 0],
            [1, 0, 0],
            [1, 0, 0],
            [1, 1, 1],
            [1, 0, 0],
            [1, 0, 0],
            [1, 0, 0],
            [1, 0, 0],
        ],
        door_ids=[
            DoorIdentifier(1, LEFT, 0, 3, 0x190A2, 0x190DE, 0),  # Beta Power Bomb Room
            DoorIdentifier(2, LEFT, 0, 5, 0x190AE, 0x1908A, 0),  # Hellway
            DoorIdentifier(3, LEFT, 0, 7, 0x19096, 0x190EA, 0),  # Alpha Power Bomb Room
            DoorIdentifier(5, RIGHT, 2, 3, 0x190C6, 0x1A480, 0),  # Red Fish Room
            DoorIdentifier(4, RIGHT, 0, 4, 0x190D2, 0x1926A, 0),  # Caterpillar Save Room
            DoorIdentifier(6, UP, 0, 0, 0x190BA, 0x18B02, None, ELEVATOR),  # Red Brinstar Elevator Room
        ],
        parts=[[0, 1, 2, 4, 5], [3]],
        transient_part_connections=[(1, 0)],  # unglitchable green gate
        missing_part_connections=[(0, 1)],
        node_tiles={
            1: [(0, 3)],  # top left door
            2: [(0, 5)],  # middle left door
            3: [(0, 6), (0, 7)],  # bottom left door
            4: [(0, 4)],  # bottom right door
            5: [(1, 3), (2, 3)],  # top right door
            6: [(0, 0), (0, 1), (0, 2)],  # elevator
            7: [(1, 3), (2, 3)],  # indirect g-mode junction
        },
    ),
    Room(
        room_id=76,
        name='Alpha Power Bomb Room',
        rom_address=0x7A3AE,
        map=[[1, 1, 1]],
        door_ids=[
            DoorIdentifier(1, RIGHT, 2, 0, 0x190EA, 0x19096, 0),  # Caterpillar Room
        ],
        items=[
            Item(1, 0, 0x7890E),
            Item(0, 0, 0x78914),
        ],
        node_tiles={
            1: [(2, 0)],  # door
            2: [(1, 0)],  # power bomb
            3: [(0, 0)],  # missile
        },
    ),
    Room(
        room_id=77,
        name='Beta Power Bomb Room',
        rom_address=0x7A37C,
        map=[
            [1, 1],
            [1, 0],
        ],
        door_ids=[
            DoorIdentifier(1, RIGHT, 1, 0, 0x190DE, 0x190A2, 0),  # Caterpillar Room
        ],
        items=[
            Item(0, 1, 0x788CA),
        ],
        node_tiles={
            1: [(0, 0), (1, 0)],  # right door
            2: [(0, 1)],  # power bomb
        },
    ),
    Room(
        # TODO: add this to wiki.supermetroid.run/List_of_rooms because it is missing.
        room_id=320,
        name='Caterpillar Save Room',
        rom_address=0x7A734,
        map=[[1]],
        door_ids=[
            DoorIdentifier(1, LEFT, 0, 0, 0x1926A, 0x190D2, 0),  # Caterpillar Room
        ],
        node_tiles={
            1: [(0, 0)],  # door
            2: [(0, 0)],  # save station
        },
    ),
    Room(
        # TODO: add this to wiki.supermetroid.run/List_of_rooms because it is missing.
        room_id=319,
        name='Sloaters Refill',
        rom_address=0x7A618,
        map=[[1]],
        door_ids=[
            DoorIdentifier(1, RIGHT, 0, 0, 0x191FE, 0x1904E, 0),  # Red Tower
        ],
        node_tiles={
            1: [(0, 0)],  # door
            2: [(0, 0)],  # refill station
        },
    ),
]


for room in rooms:
    room.area = Area.BRINSTAR
    room.sub_area = SubArea.RED_BRINSTAR

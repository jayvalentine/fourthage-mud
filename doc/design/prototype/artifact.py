import random

class WeightedTable:
    def __init__(self, items: list[str], weights: list[float]):
        if len(items) != len(weights):
            raise ValueError(f"Item ({len(items)} and weight {len(weights)} lists must be same size.")

        if abs(sum(weights) - 1) > 0.0001:
            raise ValueError("Weights must sum to 1.")

        self.items = items
        self.weights = weights

    def sample(self, roll: float):
        if roll < 0 or roll > 1:
            raise ValueError(f"Roll out of range: {roll}")

        floor = 0
        for i in range(len(self.weights)):
            if roll < self.weights[i] + floor:
                return self.items[i]
            floor += self.weights[i]

        raise RuntimeError(f"No item selected from table (roll: {roll}).")

def main():
    types = WeightedTable(['ornament', 'tome', 'weapon', 'garment', 'vessel'], [0.4, 0.2, 0.1, 0.2, 0.1])
    print(types.sample(random.uniform(0, 1)))

if __name__ == "__main__":
    main()

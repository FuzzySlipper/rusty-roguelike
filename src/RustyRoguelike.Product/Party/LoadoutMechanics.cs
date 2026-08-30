using Rusty.Engine.Entities;
using Rusty.Engine.Mechanics;
using RulesItemDefinition = RustyRoguelike.Product.Rules.ItemDefinition;

namespace RustyRoguelike.Product.Party;

/// <summary>Maps the authored initial loadout to Engine inventory/equipment relationships; game meaning remains in the party definitions.</summary>
internal sealed class LoadoutMechanics
{
    private readonly InventoryWorld _world = new();
    private readonly Dictionary<string, Rusty.Engine.Mechanics.ItemDefinition> _definitions = new(StringComparer.Ordinal);

    internal LoadoutMechanics(IEnumerable<PartyMemberState> members)
    {
        foreach (PartyMemberState member in members)
        {
            EntityId owner = new(member.Definition.EntityId);
            _world.RegisterInventory(new InventoryState(owner));
            _world.RegisterEquipment(new EquipmentState(owner));
            foreach ((RulesItemDefinition item, int index) in member.EquippedItems.Select((item, index) => (item, index)))
            {
                Rusty.Engine.Mechanics.ItemDefinition definition = Definition(item);
                EntityId itemEntity = new(checked(member.Definition.EntityId * 100 + (ulong)index + 1));
                _world.MaterializeUnique(new ItemState(itemEntity, definition), owner);
                EquipmentSlotDefinition slot = new(
                    EquipmentSlotId.Parse($"roguelike.{member.Definition.Id}.{item.Slot}"),
                    [ItemClassificationId.Parse($"roguelike.{item.Slot}")]);
                EquipmentService.Equip(_world, owner, itemEntity, [slot]);
            }
        }
    }

    internal ulong Revision => _world.Revision;

    private Rusty.Engine.Mechanics.ItemDefinition Definition(RulesItemDefinition item)
    {
        if (_definitions.TryGetValue(item.Id, out Rusty.Engine.Mechanics.ItemDefinition? definition)) return definition;
        definition = new Rusty.Engine.Mechanics.ItemDefinition(
            ItemDefinitionId.Parse($"roguelike.{item.Id}"),
            ItemKind.Unique,
            maximumQuantity: 1,
            classifications: [ItemClassificationId.Parse($"roguelike.{item.Slot}")],
            equipment: new ItemEquipmentPolicy(requiredSlots: 1),
            sourceDefinitions: [SourceDefinitionId.Parse($"roguelike.item.{item.Id}")]);
        _definitions.Add(item.Id, definition);
        return definition;
    }
}

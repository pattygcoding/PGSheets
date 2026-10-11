using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Maps cell references to their dependants so an edit can be propagated to exactly the cells
/// that need recalculating.
/// </summary>
/// <remarks>
/// Two edge directions are tracked:
/// <list type="bullet">
/// <item><description><c>precedents</c>: cell -&gt; cells its formula reads.</description></item>
/// <item><description><c>dependents</c>: cell -&gt; cells that read it (reverse index).</description></item>
/// </list>
/// Both use <see cref="CellAddress"/> keys, which hash without allocating.
/// </remarks>
public sealed class DependencyGraph
{
    private readonly Dictionary<CellAddress, HashSet<CellAddress>> _precedents = new();
    private readonly Dictionary<CellAddress, HashSet<CellAddress>> _dependents = new();

    public void Clear()
    {
        _precedents.Clear();
        _dependents.Clear();
    }

    public bool TryGetPrecedents(CellAddress cell, out HashSet<CellAddress> precedents) =>
        _precedents.TryGetValue(cell, out precedents!);

    /// <summary>
    /// Replaces the outgoing edges of <paramref name="cell"/>. Called every time a formula is
    /// evaluated, which keeps the graph correct as formulas are edited.
    /// </summary>
    public void SetPrecedents(CellAddress cell, IReadOnlyCollection<CellAddress> precedents)
    {
        if (_precedents.TryGetValue(cell, out var existing))
        {
            foreach (var precedent in existing)
            {
                if (_dependents.TryGetValue(precedent, out var dependents))
                {
                    dependents.Remove(cell);
                    if (dependents.Count == 0)
                    {
                        _dependents.Remove(precedent);
                    }
                }
            }

            existing.Clear();
        }

        if (precedents.Count == 0)
        {
            _precedents.Remove(cell);
            return;
        }

        if (!_precedents.TryGetValue(cell, out var target))
        {
            target = new HashSet<CellAddress>();
            _precedents[cell] = target;
        }

        foreach (var precedent in precedents)
        {
            if (!target.Add(precedent))
            {
                continue;
            }

            if (!_dependents.TryGetValue(precedent, out var dependents))
            {
                dependents = new HashSet<CellAddress>();
                _dependents[precedent] = dependents;
            }

            dependents.Add(cell);
        }
    }

    /// <summary>
    /// Detaches a deleted cell's outgoing edges. Incoming edges are deliberately kept because other
    /// cells may still reference the address (they will see it as empty and must recalculate).
    /// </summary>
    public void RemoveCell(CellAddress cell)
    {
        if (!_precedents.TryGetValue(cell, out var precedents))
        {
            return;
        }

        foreach (var precedent in precedents)
        {
            if (_dependents.TryGetValue(precedent, out var dependents))
            {
                dependents.Remove(cell);
                if (dependents.Count == 0)
                {
                    _dependents.Remove(precedent);
                }
            }
        }

        _precedents.Remove(cell);
    }

    /// <summary>
    /// Breadth-first collection of <paramref name="start"/> plus everything reachable through
    /// dependent edges; the set of cells affected by an edit.
    /// </summary>
    public void CollectAffected(CellAddress start, HashSet<CellAddress> into)
    {
        into.Add(start);

        var queue = new Queue<CellAddress>();
        queue.Enqueue(start);

        while (queue.Count > 0)
        {
            var current = queue.Dequeue();

            if (!_dependents.TryGetValue(current, out var dependents))
            {
                continue;
            }

            foreach (var dependent in dependents)
            {
                if (into.Add(dependent))
                {
                    queue.Enqueue(dependent);
                }
            }
        }
    }
}

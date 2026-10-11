using PGSheets.Core;
using PGSheets.Formulas;

namespace PGSheets.Tests;

public class DependencyGraphTests
{
    private static readonly CellAddress A = new(0, 0);
    private static readonly CellAddress B = new(0, 1);
    private static readonly CellAddress C = new(0, 2);

    [Fact]
    public void Collect_affected_walks_dependants_transitively()
    {
        var graph = new DependencyGraph();

        graph.SetPrecedents(B, new[] { A });
        graph.SetPrecedents(C, new[] { B });

        var affected = new HashSet<CellAddress>();
        graph.CollectAffected(A, affected);

        Assert.Equal(3, affected.Count);
        Assert.Contains(A, affected);
        Assert.Contains(B, affected);
        Assert.Contains(C, affected);
    }

    [Fact]
    public void Collect_affected_starts_with_the_changed_cell()
    {
        var graph = new DependencyGraph();
        var affected = new HashSet<CellAddress>();

        graph.CollectAffected(A, affected);

        Assert.Single(affected);
        Assert.Contains(A, affected);
    }

    [Fact]
    public void Replacing_precedents_detaches_the_stale_edges()
    {
        var graph = new DependencyGraph();
        var target = new CellAddress(1, 0);

        graph.SetPrecedents(target, new[] { A });
        graph.SetPrecedents(target, new[] { B });

        var fromA = new HashSet<CellAddress>();
        graph.CollectAffected(A, fromA);
        Assert.Single(fromA);

        var fromB = new HashSet<CellAddress>();
        graph.CollectAffected(B, fromB);
        Assert.Contains(target, fromB);

        Assert.True(graph.TryGetPrecedents(target, out var precedents));
        Assert.DoesNotContain(A, precedents);
        Assert.Contains(B, precedents);
    }

    [Fact]
    public void Removing_a_cell_drops_its_outgoing_edges_but_keeps_dependants()
    {
        var graph = new DependencyGraph();

        graph.SetPrecedents(A, new[] { B }); // A is a formula reading B
        graph.SetPrecedents(C, new[] { A }); // C is a formula reading A

        graph.RemoveCell(A);

        Assert.False(graph.TryGetPrecedents(A, out _));

        // C must still recalculate because it references the now-empty A.
        var fromA = new HashSet<CellAddress>();
        graph.CollectAffected(A, fromA);
        Assert.Contains(C, fromA);

        // A is no longer a dependant of B.
        var fromB = new HashSet<CellAddress>();
        graph.CollectAffected(B, fromB);
        Assert.DoesNotContain(A, fromB);
    }

    [Fact]
    public void A_cycle_does_not_hang_collection()
    {
        var graph = new DependencyGraph();

        graph.SetPrecedents(A, new[] { B });
        graph.SetPrecedents(B, new[] { A });

        var affected = new HashSet<CellAddress>();
        graph.CollectAffected(A, affected);

        Assert.Equal(2, affected.Count);
    }

    [Fact]
    public void Clearing_the_graph_removes_every_edge()
    {
        var graph = new DependencyGraph();

        graph.SetPrecedents(B, new[] { A });
        graph.Clear();

        var affected = new HashSet<CellAddress>();
        graph.CollectAffected(A, affected);

        Assert.Single(affected);
        Assert.False(graph.TryGetPrecedents(B, out _));
    }
}

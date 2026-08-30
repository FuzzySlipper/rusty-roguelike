using Rusty.Engine;
using RustyRoguelike.Product.Presentation;
using RustyRoguelike.Product.Session;

namespace RustyRoguelike.Product;

/// <summary>
/// Engine-admitted lifecycle boundary for the roguelike's future product domains.
/// Rules, floor admission, sessions, and presentation attach here without owning a loop.
/// </summary>
public sealed class RoguelikeProduct : IEngineProduct
{
    private readonly LifecycleProjection _lifecycleProjection;
    private readonly GameSessionProjection _sessionProjection;
    private readonly IRandomService _random;
    private GameSession _session;
    private bool _started;
    private bool _paused;
    private bool _shutdown;

    public RoguelikeProduct(ProductCreateContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        _lifecycleProjection = new LifecycleProjection(context.Engine.Ui);
        _sessionProjection = new GameSessionProjection(context.Engine.Ui);
        _random = context.Engine.Random;
        _session = new GameSession(_random);
        _lifecycleProjection.Publish(LifecycleSnapshot.Created);
        _sessionProjection.Publish(_session);
    }

    public void Start()
    {
        if (_shutdown)
        {
            return;
        }

        _started = true;
        _paused = false;
        _lifecycleProjection.Publish(LifecycleSnapshot.Started);
        _sessionProjection.Publish(_session);
    }

    public ProductUpdateResult Update(ProductUpdate update)
    {
        if (!_started || _paused || _shutdown)
        {
            return ProductUpdateResult.None;
        }

        LifecycleSnapshot snapshot = LifecycleSnapshot.From(update.Facts, update.Input.Length);
        _lifecycleProjection.Publish(snapshot);
        _sessionProjection.Publish(_session);
        return ProductUpdateResult.None;
    }

    public void Pause()
    {
        if (!_started || _shutdown)
        {
            return;
        }

        _paused = true;
        _lifecycleProjection.Publish(LifecycleSnapshot.Paused);
        _sessionProjection.Publish(_session);
    }

    public void Resume()
    {
        if (!_started || _shutdown)
        {
            return;
        }

        _paused = false;
        _lifecycleProjection.Publish(LifecycleSnapshot.Resumed);
        _sessionProjection.Publish(_session);
    }

    public void Restart()
    {
        if (_shutdown)
        {
            return;
        }

        _started = true;
        _paused = false;
        _session = new GameSession(_random);
        _lifecycleProjection.Publish(LifecycleSnapshot.Restarted);
        _sessionProjection.Publish(_session);
    }

    public void Shutdown()
    {
        if (_shutdown)
        {
            return;
        }

        _lifecycleProjection.Dispose();
        _sessionProjection.Dispose();
        _shutdown = true;
    }

    /// <summary>Future strict transport commands enter through this single product-owned revision boundary.</summary>
    internal SessionCommandReceipt Submit(SessionCommand command)
    {
        if (!_started || _paused || _shutdown)
        {
            return new SessionCommandReceipt(false, "inactive-lifecycle", _session.Revision, []);
        }

        SessionCommandReceipt receipt = _session.Submit(command);
        _sessionProjection.Publish(_session);
        return receipt;
    }

    public void Dispose() => Shutdown();
}

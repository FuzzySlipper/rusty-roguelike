using Rusty.Engine;
using RustyRoguelike.Product.Presentation;

namespace RustyRoguelike.Product;

/// <summary>
/// Engine-admitted lifecycle boundary for the roguelike's future product domains.
/// Rules, floor admission, sessions, and presentation attach here without owning a loop.
/// </summary>
public sealed class RoguelikeProduct : IEngineProduct
{
    private readonly LifecycleProjection _lifecycleProjection;
    private bool _started;
    private bool _paused;
    private bool _shutdown;

    public RoguelikeProduct(ProductCreateContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        _lifecycleProjection = new LifecycleProjection(context.Engine.Ui);
        _lifecycleProjection.Publish(LifecycleSnapshot.Created);
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
    }

    public ProductUpdateResult Update(ProductUpdate update)
    {
        if (!_started || _paused || _shutdown)
        {
            return ProductUpdateResult.None;
        }

        LifecycleSnapshot snapshot = LifecycleSnapshot.From(update.Facts, update.Input.Length);
        _lifecycleProjection.Publish(snapshot);
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
    }

    public void Resume()
    {
        if (!_started || _shutdown)
        {
            return;
        }

        _paused = false;
        _lifecycleProjection.Publish(LifecycleSnapshot.Resumed);
    }

    public void Restart()
    {
        if (_shutdown)
        {
            return;
        }

        _started = true;
        _paused = false;
        _lifecycleProjection.Publish(LifecycleSnapshot.Restarted);
    }

    public void Shutdown()
    {
        if (_shutdown)
        {
            return;
        }

        _lifecycleProjection.Dispose();
        _shutdown = true;
    }

    public void Dispose() => Shutdown();
}

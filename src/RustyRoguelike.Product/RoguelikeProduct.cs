using Rusty.Engine;
using Rusty.Engine.Persistence;
using RustyRoguelike.Product.Integration;
using RustyRoguelike.Product.Presentation;
using RustyRoguelike.Product.Saves;
using RustyRoguelike.Product.Session;
using System.Text;

namespace RustyRoguelike.Product;

/// <summary>
/// Engine-admitted lifecycle boundary for the roguelike's future product domains.
/// Rules, floor admission, sessions, and presentation attach here without owning a loop.
/// </summary>
public sealed class RoguelikeProduct : IEngineProduct
{
    private readonly LifecycleProjection _lifecycleProjection;
    private readonly GameSessionProjection _sessionProjection;
    private readonly EngineIntegrationProjection _integrationProjection;
    private readonly IRandomService _random;
    private readonly FloorEngineProjection _floor;
    private readonly RoguelikeSaveStore _saves;
    private GameSession _session;
    private SaveOperationReadout _saveReadout = SaveOperationReadout.None;
    private bool _started;
    private bool _paused;
    private bool _shutdown;

    public RoguelikeProduct(ProductCreateContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        LifecycleProjection? lifecycle = null;
        GameSessionProjection? sessionProjection = null;
        EngineIntegrationProjection? integrationProjection = null;
        FloorEngineProjection? floor = null;
        RoguelikeSaveStore? saves = null;
        try
        {
            lifecycle = new LifecycleProjection(context.Engine.Ui);
            sessionProjection = new GameSessionProjection(context.Engine.Ui);
            integrationProjection = new EngineIntegrationProjection(context.Engine.Ui);
            floor = FloorEngineProjection.Create(context.Engine);
            saves = new RoguelikeSaveStore(context.Engine);
            IRandomService random = context.Engine.Random;
            GameSession session = new(random, floor.Floor, floor.QueryVisibleOpposition, floor.ProposePartyStep);

            _lifecycleProjection = lifecycle;
            _sessionProjection = sessionProjection;
            _integrationProjection = integrationProjection;
            _floor = floor;
            _saves = saves;
            _random = random;
            _session = session;
            _lifecycleProjection.Publish(LifecycleSnapshot.Created);
            Publish();
        }
        catch
        {
            saves?.Dispose();
            floor?.Dispose();
            integrationProjection?.Dispose();
            sessionProjection?.Dispose();
            lifecycle?.Dispose();
            throw;
        }
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
        Publish();
    }

    public ProductUpdateResult Update(ProductUpdate update)
    {
        if (!_started || _paused || _shutdown)
        {
            return ProductUpdateResult.None;
        }

        LifecycleSnapshot snapshot = LifecycleSnapshot.From(update.Facts, update.Input.Length);
        _lifecycleProjection.Publish(snapshot);
        foreach (ProductInputEvent input in update.Input)
        {
            HandleInput(input);
        }
        Publish();
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
        Publish();
    }

    public void Resume()
    {
        if (!_started || _shutdown)
        {
            return;
        }

        _paused = false;
        _lifecycleProjection.Publish(LifecycleSnapshot.Resumed);
        Publish();
    }

    public void Restart()
    {
        if (_shutdown)
        {
            return;
        }

        _started = true;
        _paused = false;
        _session = NewSession();
        _lifecycleProjection.Publish(LifecycleSnapshot.Restarted);
        Publish();
    }

    public void Shutdown()
    {
        if (_shutdown)
        {
            return;
        }

        _lifecycleProjection.Dispose();
        _sessionProjection.Dispose();
        _integrationProjection.Dispose();
        _saves.Dispose();
        _floor.Dispose();
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
        Publish();
        return receipt;
    }

    private GameSession NewSession() => new(_random, _floor.Floor, _floor.QueryVisibleOpposition, _floor.ProposePartyStep);

    private void HandleInput(ProductInputEvent input)
    {
        if (input.Kind != InputEventKind.DirectDigital
            || input.Provenance != InputProvenance.DirectUi
            || input.ValueKind != InputValueKind.Digital
            || input.X != 1.0f)
        {
            return;
        }

        string intent = Encoding.UTF8.GetString(input.Intent.Span);
        switch (intent)
        {
            case "roguelike.begin":
                Submit(new BeginExpeditionCommand(_session.Revision));
                break;
            case "roguelike.move.north":
                Submit(new MovePartyCommand(_session.Revision, 0, -1));
                break;
            case "roguelike.move.east":
                Submit(new MovePartyCommand(_session.Revision, 1, 0));
                break;
            case "roguelike.move.south":
                Submit(new MovePartyCommand(_session.Revision, 0, 1));
                break;
            case "roguelike.move.west":
                Submit(new MovePartyCommand(_session.Revision, -1, 0));
                break;
            case "roguelike.wait":
                Submit(new WaitCommand(_session.Revision));
                break;
            case "roguelike.save":
                Save();
                break;
            case "roguelike.load":
                Load();
                break;
        }
    }

    private void Save()
    {
        try
        {
            PersistenceSaveReceipt saved = _saves.Save(_session, _floor);
            _saveReadout = new SaveOperationReadout("save", "accepted", saved.Revision, "closed product snapshot saved through Engine persistence");
        }
        catch (Exception exception)
        {
            _saveReadout = new SaveOperationReadout("save", "rejected", 0, exception.Message);
        }
    }

    private void Load()
    {
        try
        {
            ProductStateLoad<RoguelikeSave> loaded = _saves.Load();
            if (!loaded.Present || loaded.State is null)
            {
                _saveReadout = new SaveOperationReadout("load", "absent", loaded.Revision, "the Engine store has no saved session");
                return;
            }
            RoguelikeSave save = loaded.State;
            if (save.FloorProvenance != _floor.Floor.Provenance || save.FloorContentHash != _floor.Content.Sha256)
            {
                throw new InvalidOperationException("save-floor-provenance-mismatch");
            }
            _session = GameSession.Restore(_random, _floor.Floor, save.Session, _floor.QueryVisibleOpposition, _floor.ProposePartyStep);
            _saveReadout = new SaveOperationReadout("load", "accepted", loaded.Revision, "closed product snapshot restored after floor provenance validation");
        }
        catch (Exception exception)
        {
            _saveReadout = new SaveOperationReadout("load", "rejected", 0, exception.Message);
        }
    }

    private void Publish()
    {
        _floor.RefreshReadout();
        _sessionProjection.Publish(_session);
        _integrationProjection.Publish(_floor.Readout, _saveReadout);
    }

    public void Dispose() => Shutdown();
}

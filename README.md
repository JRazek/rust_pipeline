# Dynamic pipeline

Introduces an interface for building pipelines in consistent way.
Crate was inspired by gstreamer pads and the way it builds pipeline, but in more Rust-friendly way and fully exploiting Rust trait system.

## Pads

Dynamic pipeline consists of 3 types of elements intertwined together.

1. `StreamPad` acting as a data producer.
2. `SinkPad` acting as a data consumer.
3. `LinkElement` acting as an element that joints together Streams and Sinks.

In the process of pipeline building Link is decomposed into (Sink, Stream) tuple acting as 2 ends of a link. Data flow between these 2 is implemented by the end user.

  Note that (Sink, Stream) order is reversed here.
  Link element acts as a bridge between the two pads.

```
  Stream ---> [Sink ---> Stream] ---> ... --> ... ---> Sink
              ^^^^^^Link^^^^^^^^      other links
```

Typically in the context of link inner implementation, exposed sink acts as a producer for the data that exposed stream receives. The opposite of what is seen by the pipeline.


### Format negotiation

To ensure correctness of sent data between the elements in dynamic way. Crate borrows a concept of format negotiation from gstreamer.

There are 2 traits used for the data negotiation:

```rust
pub trait FormatProvider<F> {
    fn formats(&self) -> Vec<F>;
}

pub trait FormatNegotiator<F> {
    fn matches(&self, format: &F) -> bool;
}
```

Process of pipeline building with pipeline negotiator::Builder object, consists of specifying an initial Stream in pipeline, adding links (if any) and ending the pipeline with a sink.

In negotiator::Builder a stream needs to implement both: SinkPad and FormatProvider. Sink has to implement a FormatNegotiator on the other hand.

FormatNegotiator::matches is a predicate used to determine a subset of formats returned by the FormatProvider::formats.
Then, during connecting each pad, the first format that satisfies the FormatProvider::matches is chosen as a format for the connection.

Actual definition of these traits:
```rust
pub trait StreamPad<D, F> {
    type Receiver: Receiver<D>;

    fn get_rx(self, rx_format: &F) -> Result<Self::Receiver, LinkError>;
}

pub trait SinkPad<D, F> {
    type Sender: Sender<D>;

    fn get_tx(self, tx_format: &F) -> Result<Self::Sender, LinkError>;
}
```
Note the exposure of actual Receiver/Sender. These funcitons have to expose these in order for the negotiator::Builder to actually connect each element.
Each of these also take the rx/tx formats in order to prepare the element.

#### Links

Links are a bit more complex. Suppose that we want to create an element that exposes the formats based on the format that was chosen for the communication with the previous element.

e.g. a simple passthrough element that may observe the data, but not interfere with it.

```
| Stream {μ-law, pcm} | --> | P(x) [Sink, Stream] {??} | --> | Q(x) Stream |

```

where, P(x) := true for all x and Q(x) is arbitrary.

Set {} on the right of the element denotes the set of provided formats by FormatProvider and P, Q are predicates of FormatNegotiator.

Based on the chosen format for the communication we want to substitute {??} with some actual values.

and so the LinkElement trait is defined as:

```rust
pub trait LinkElement<Drx, Frx, Dtx = Drx, Ftx = Frx> {
    type SinkPad: SinkPad<Drx, Frx>;
    type StreamPad: StreamPad<Dtx, Ftx>;

    fn get_pads(self, rx_format: &Frx) -> Result<(Self::SinkPad, Self::StreamPad), LinkError>;
}
```
negotiator::Builder builder also requires LinkElement to implement FormatNegotiator trait and LinkElement::StreamPad to implement FormatProvider trait. Then we may construct the pads once the format of the communication is already known. In this example we would construct a stream pad with the provided formats set equal to {rx_format}. Then the tuple is deconstucted and the Stream is used in the same manner as the former stream in pipeline.

Once the traits are implemented, connecting the traits is quite straight-forward.
```rust
fn main() {
    use pipeline::pipeline_builder::negotiator::Builder;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let producer_stream_pad = audio_producer(&rt);

    let passthrough_link = PassthroughLink { rt: &rt };

    let consumer_pad = consumer_task(&rt);

    negotiator::Builder::with_stream(producer_stream_pad)
        .set_link(passthrough_link, &rt)
        .unwrap()
        .build_with_sink(consumer_pad, &rt)
        .unwrap();

    rt.block_on(async {
        tokio::signal::ctrl_c().await.unwrap();
    });
}
```

## Manual linking
One may also choose to manually link the elements without using any form of format negotiation. In this case we do not have implement FormatProvider or FormatNegotiator anywhere and use the manual::Builder in the same way as negotiator::Builder, but with specifying a format at each joint.

<3 Unux

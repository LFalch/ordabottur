use reqwest::{
    header::CONTENT_TYPE,
    get as reqwest_get,
    Client as ReqClient,
    RequestBuilder,
};
use scraper::{Html, Selector};
use encoding_rs::mem::convert_utf8_to_latin1_lossy;

use crate::util::Entry;
pub async fn gm_entries(ord: &str, result_row_amount: u16) -> Result<(String, Vec<Entry>), u16> {
    let client = ReqClient::new();

    let res = gm_post(client.post("http://www.edd.uio.no/perl/search/search.cgi"),
        ord, result_row_amount)
        .send()
        .await
        .unwrap();

    if res.status().is_success() {
        let html = Html::parse_document(&res.text().await.unwrap());

        let entry_selector = Selector::parse(".ResRowGray td, .ResRowWhite td").unwrap();
        let result_number_selector = Selector::parse(".BeneathNavigator").unwrap();

        let mut iter = html.select(&entry_selector).map(|tr| tr.inner_html().trim().to_owned());

        let mut entries = Vec::with_capacity(iter.size_hint().0);

        while let Some(word) = iter.next() {
            let class = iter.next().unwrap();
            let body = iter.next().unwrap();

            entries.push(Entry::new_gm(word, class, body));
        }

        // HACK don't look at this
        let results = html.select(&result_number_selector).next().unwrap().text().next().unwrap().to_owned();

        Ok((results, entries))
    } else {
        Err(res.status().as_u16())
    }
}

pub async fn sa_entries<'a, 'b>(ord: &'a str, result_row_amount: u16, options: SetelArkivOptions<'b>) -> Result<(String, Vec<Entry>), u16> {
    let client = ReqClient::new();

    let res = sa_post(client.post("http://www.edd.uio.no/perl/search/search.cgi"),
        ord, result_row_amount, options)
        .send()
        .await
        .unwrap();

    if res.status().is_success() {
        let html = Html::parse_document(&res.text().await.unwrap());

        let entry_selector = Selector::parse(".ResRowGray td, .ResRowWhite td").unwrap();
        let result_number_selector = Selector::parse(".BeneathNavigator").unwrap();

        let mut iter = html.select(&entry_selector).map(|tr| tr.inner_html().trim().to_owned());

        let mut entries = Vec::with_capacity(iter.size_hint().0);

        while let Some(word) = iter.next() {
            let class = iter.next().unwrap();
            let body = iter.next().unwrap();

            entries.push(Entry::new_gm(word, class, body));
        }

        // HACK don't look at this
        let results = html.select(&result_number_selector).next().unwrap().text().next().unwrap().to_owned();

        Ok((results, entries))
    } else {
        Err(res.status().as_u16())
    }
}
pub async fn sa_entry(id: u32) -> Result<(String, Option<String>), u16> {
    let res = reqwest_get(&format!("https://www.edd.uio.no/perl/search/objectviewer.cgi?tabid=436&primarykey={}", id)).await.unwrap();

    if res.status().is_success() {
        let html = Html::parse_document(&res.text().await.unwrap());

        let oppslag_selector = Selector::parse(".oppslag").unwrap();
        let grammar_selector = Selector::parse(".GRAMMATIKK").unwrap();
        let img_selector = Selector::parse("img").unwrap();

        let oppslag = html.select(&oppslag_selector).next().unwrap().inner_html().trim().to_owned();
        let grammar = html.select(&grammar_selector).next().unwrap().inner_html().trim().to_owned();
        
        match html.select(&img_selector).next().map(|img| img.value().attr("src").unwrap_or("/")) {
            Some(url) => {
                let img = format!("https://www.edd.uio.no{}", url);
                Ok((format!("**{}** ({})", oppslag, grammar), Some(img)))
            }
            None => {
                let kontekst_selector = Selector::parse(".kontekst").unwrap();
                let kontekst = html.select(&kontekst_selector).next().unwrap().inner_html().trim().to_owned();

                Ok((format!("**{}** ({})\n_{}_", oppslag, grammar, kontekst), None))
            }
        }
    } else {
        Err(res.status().as_u16())
    }
}

fn gm_post(rb: RequestBuilder, word: &str, result_row_amount: u16) -> RequestBuilder {
    rb
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!("tabid=993&appid=59&C%23993.994.545%23994.995.546%23ORD={}&dosearch=++++S%F8k++++&oppsetttid=215&ResultatID=447&ResRowsNum={}",
            uio_encode(word), result_row_amount))
}

#[derive(Copy, Clone, Debug, Default)]
pub struct SetelArkivOptions<'a> {
    pub registrant: &'a str, 
    pub title: &'a str,
    pub author: &'a str,
    pub area_code: &'a str,
    pub place_code: &'a str,
}

fn sa_post(rb: RequestBuilder, word_form: &str, result_row_amount: u16, options: SetelArkivOptions) -> RequestBuilder {
    let SetelArkivOptions {
        registrant,
        title,
        author,
        area_code,
        place_code,
    } = options;

    rb
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!("tabid=436&appid=8&C%23436.437.235%23ORDFORM={}&C%23436.447.243%23PERSONNAMN={}&C%23436.443.239%23443.444.240%23FORFATTAR={}\
                        &C%23436.443.239%23443.444.240%23TITTEL={}&C%23436.1855.1051%231855.448.1050%23STADNAMNKODE={}&C%23436.635.339%23635.448.341%23STADNAMNKODE={}\
                        &C%23SETEL_ID=&dosearch=++++S%F8k++++&oppsettid=216&ResultatID=328&ResRowsNum={}",
            uio_encode(word_form), uio_encode(registrant), uio_encode(author), uio_encode(title), uio_encode(area_code), uio_encode(place_code), result_row_amount))
}

fn uio_encode(s: &str) -> String {
    let mut buf_bytes =  vec![0; s.len()];

    convert_utf8_to_latin1_lossy(s.as_bytes(), &mut buf_bytes);

    let mut encoded = String::with_capacity(buf_bytes.len());

    for byte in buf_bytes {
        match byte {
            0x00 => (),
            0x01..=0x1f | 0x21..=0x2c | 0x3a..=0x40 | 0x5b..=0x60 | 0x7b..=0xff => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
            0x20 => encoded.push('+'),
            0x2d..=0x39 | 0x41..=0x5a | 0x61..=0x7a => encoded.push(byte as char),
        }
    }

    encoded
}